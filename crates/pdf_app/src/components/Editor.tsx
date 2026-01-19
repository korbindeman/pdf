import {
  useRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  forwardRef,
} from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap, placeholder } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { gruvboxDark } from "cm6-theme-gruvbox-dark";

interface EditorProps {
  value: string;
  onChange: (value: string) => void;
}

export interface EditorHandle {
  insertPageBreak: () => void;
}

// Custom theme overrides for gruvbox
const customTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px",
    backgroundColor: "#282828",
  },
  ".cm-scroller": {
    fontFamily:
      "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
    lineHeight: "1.6",
    padding: "16px",
  },
  ".cm-content": {
    caretColor: "#ebdbb2",
  },
  ".cm-cursor": {
    borderLeftColor: "#ebdbb2",
  },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
    backgroundColor: "#504945",
  },
  ".cm-activeLine": {
    backgroundColor: "transparent",
  },
  ".cm-gutters": {
    display: "none",
  },
});

export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  { value, onChange },
  ref,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const isExternalUpdate = useRef(false);

  // Initialize CodeMirror
  useEffect(() => {
    if (!containerRef.current) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !isExternalUpdate.current) {
        onChange(update.state.doc.toString());
      }
      isExternalUpdate.current = false;
    });

    const state = EditorState.create({
      doc: value,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        markdown(),
        gruvboxDark,
        customTheme,
        placeholder("Write markdown here..."),
        updateListener,
        EditorView.lineWrapping,
      ],
    });

    const view = new EditorView({
      state,
      parent: containerRef.current,
    });

    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, []); // Only run once on mount

  // Sync external value changes
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;

    const currentValue = view.state.doc.toString();
    if (value !== currentValue) {
      isExternalUpdate.current = true;
      view.dispatch({
        changes: {
          from: 0,
          to: currentValue.length,
          insert: value,
        },
      });
    }
  }, [value]);

  // Insert page break at cursor
  const insertPageBreak = useCallback(() => {
    const view = viewRef.current;
    if (!view) return;

    const { state } = view;
    const pos = state.selection.main.head;

    // Find end of current line
    const line = state.doc.lineAt(pos);
    const insertPos = line.to;

    // Insert pagebreak on new line
    const insertText = "\n---pagebreak---\n";

    view.dispatch({
      changes: { from: insertPos, insert: insertText },
      selection: { anchor: insertPos + insertText.length },
    });

    view.focus();
  }, []);

  // Expose methods to parent
  useImperativeHandle(
    ref,
    () => ({
      insertPageBreak,
    }),
    [insertPageBreak],
  );

  return (
    <div
      ref={containerRef}
      className="w-full h-full overflow-auto"
      style={{ backgroundColor: "#282828" }}
    />
  );
});
