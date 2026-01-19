import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile, watch } from "@tauri-apps/plugin-fs";
import { Editor, EditorHandle } from "./components/Editor";
import { PdfViewer } from "./components/PdfViewer";
import {
  getLastOpenDir,
  setLastOpenDir,
  getLastSaveDir,
  setLastSaveDir,
} from "./lib/paths";

const AUTO_SAVE_DELAY = 2000; // Auto-save after 2 seconds of inactivity

const SAMPLE_MARKDOWN = `# Hello World

This is a **live preview** of your markdown rendered as a PDF.

## Features

- Edit markdown on the left
- See the PDF on the right
- Navigate between pages

## Code Example

\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`
`;

interface SvgDocument {
  pages: string[];
  width_pt: number;
  height_pt: number;
}

function getDefaultPdfName(currentFile: string | null): string {
  if (!currentFile) return "document.pdf";
  const name =
    currentFile.split("/").pop() || currentFile.split("\\").pop() || "document";
  return name.replace(/\.(md|markdown|txt)$/i, "") + ".pdf";
}

function App() {
  const [markdown, setMarkdown] = useState(SAMPLE_MARKDOWN);
  const [svgDoc, setSvgDoc] = useState<SvgDocument | null>(null);
  const [currentFile, setCurrentFile] = useState<string | null>(null);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [autoSaveEnabled, _setAutoSaveEnabled] = useState(true); // TODO: expose in config UI
  const debounceRef = useRef<number | null>(null);
  const autoSaveRef = useRef<number | null>(null);
  const markdownRef = useRef(markdown);
  const currentFileRef = useRef<string | null>(null);
  const savedContentRef = useRef<string | null>(null); // Track last saved content
  const isExternalChangeRef = useRef(false); // Track if change came from file watcher
  const editorRef = useRef<EditorHandle>(null);

  // Keep refs in sync with state
  useEffect(() => {
    markdownRef.current = markdown;
  }, [markdown]);

  useEffect(() => {
    currentFileRef.current = currentFile;
  }, [currentFile]);

  // Save markdown file to disk
  const saveMarkdown = useCallback(async () => {
    const file = currentFileRef.current;
    const content = markdownRef.current;
    if (!file) return;

    try {
      await writeTextFile(file, content);
      savedContentRef.current = content;
      setHasUnsavedChanges(false);
    } catch (err) {
      console.error("Failed to save file:", err);
    }
  }, []);

  const openFile = useCallback(async () => {
    const lastDir = getLastOpenDir();
    const selected = await open({
      multiple: false,
      defaultPath: lastDir,
      filters: [{ name: "Markdown", extensions: ["md", "markdown", "txt"] }],
    });

    if (selected) {
      setLastOpenDir(selected);
      const content = await readTextFile(selected);
      savedContentRef.current = content;
      isExternalChangeRef.current = true;
      setMarkdown(content);
      setCurrentFile(selected);
      setHasUnsavedChanges(false);
    }
  }, []);

  const savePdf = useCallback(async () => {
    const currentMarkdown = markdownRef.current;
    if (!currentMarkdown.trim()) return;

    const lastDir = getLastSaveDir();
    const defaultName = getDefaultPdfName(currentFileRef.current);
    const defaultPath = lastDir ? `${lastDir}/${defaultName}` : defaultName;

    const path = await save({
      filters: [{ name: "PDF", extensions: ["pdf"] }],
      defaultPath,
    });

    if (path) {
      setLastSaveDir(path);
      await invoke("save_pdf_to_file", { markdown: currentMarkdown, path });
    }
  }, []);

  // Listen for menu events
  useEffect(() => {
    const unlistenOpen = listen("menu-open-file", () => {
      openFile();
    });

    const unlistenSave = listen("menu-save-pdf", () => {
      savePdf();
    });

    return () => {
      unlistenOpen.then((fn) => fn());
      unlistenSave.then((fn) => fn());
    };
  }, [openFile, savePdf]);

  // Handle Cmd+S to save markdown
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        saveMarkdown();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [saveMarkdown]);

  // Watch file for external changes
  useEffect(() => {
    if (!currentFile) return;

    let unwatchFn: (() => void) | null = null;

    const setupWatch = async () => {
      try {
        unwatchFn = await watch(currentFile, async (event) => {
          // Only reload on modify events, not on our own saves
          if (
            event.type &&
            typeof event.type === "object" &&
            "modify" in event.type
          ) {
            try {
              const content = await readTextFile(currentFile);
              // Only update if content is different (avoids loop from our own saves)
              if (content !== markdownRef.current) {
                savedContentRef.current = content;
                isExternalChangeRef.current = true;
                setMarkdown(content);
                setHasUnsavedChanges(false);
              }
            } catch (err) {
              console.error("Failed to reload file:", err);
            }
          }
        });
      } catch (err) {
        console.error("Failed to watch file:", err);
      }
    };

    setupWatch();

    return () => {
      if (unwatchFn) {
        unwatchFn();
      }
    };
  }, [currentFile]);

  // Track unsaved changes and auto-save
  useEffect(() => {
    // Skip if this change came from file watcher or opening a file
    if (isExternalChangeRef.current) {
      isExternalChangeRef.current = false;
      return;
    }

    // Check if content differs from saved version
    if (currentFile && savedContentRef.current !== null) {
      const isDirty = markdown !== savedContentRef.current;
      setHasUnsavedChanges(isDirty);

      // Auto-save if enabled and there are unsaved changes
      if (autoSaveEnabled && isDirty) {
        if (autoSaveRef.current) {
          clearTimeout(autoSaveRef.current);
        }
        autoSaveRef.current = window.setTimeout(() => {
          saveMarkdown();
        }, AUTO_SAVE_DELAY);
      }
    }

    return () => {
      if (autoSaveRef.current) {
        clearTimeout(autoSaveRef.current);
      }
    };
  }, [markdown, currentFile, autoSaveEnabled, saveMarkdown]);

  // Convert markdown to SVG for preview
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    debounceRef.current = window.setTimeout(async () => {
      if (!markdown.trim()) {
        setSvgDoc(null);
        return;
      }

      try {
        const doc = await invoke<SvgDocument>("render_markdown_to_svg", {
          markdown,
        });
        setSvgDoc(doc);
      } catch (err) {
        console.error("Failed to render:", err);
      }
    }, 300);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [markdown]);

  // Get just the filename from the path
  const fileName = currentFile
    ? currentFile.split("/").pop() || currentFile.split("\\").pop()
    : null;

  return (
    <div
      className="h-screen flex overflow-hidden"
      style={{ backgroundColor: "#282828", color: "#ebdbb2" }}
    >
      <div
        className="w-1/2 min-w-0 flex flex-col relative"
        style={{ borderRight: "1px solid #3c3836" }}
      >
        <Editor ref={editorRef} value={markdown} onChange={setMarkdown} />
        <div className="absolute bottom-3 left-3 flex items-center gap-2">
          {fileName && (
            <div
              className="px-2 py-1 backdrop-blur-sm rounded text-xs truncate max-w-48"
              style={{
                backgroundColor: "rgba(60, 56, 54, 0.9)",
                color: "#a89984",
              }}
            >
              {fileName}
              {hasUnsavedChanges && (
                <span style={{ color: "#665c54" }} className="ml-0.5">
                  *
                </span>
              )}
            </div>
          )}
          <button
            onClick={() => editorRef.current?.insertPageBreak()}
            className="px-2 py-1 backdrop-blur-sm rounded text-xs transition-colors hover:brightness-110"
            style={{
              backgroundColor: "rgba(60, 56, 54, 0.9)",
              color: "#a89984",
            }}
            title="Insert page break"
          >
            Page Break
          </button>
        </div>
      </div>
      <div className="w-1/2 min-w-0 flex flex-col">
        <PdfViewer svgDoc={svgDoc} onSave={savePdf} />
      </div>
    </div>
  );
}

export default App;
