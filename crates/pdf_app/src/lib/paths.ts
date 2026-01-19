const OPEN_DIR_KEY = "pdf_app_open_dir";
const SAVE_DIR_KEY = "pdf_app_save_dir";

export function getLastOpenDir(): string | undefined {
  return localStorage.getItem(OPEN_DIR_KEY) || undefined;
}

export function setLastOpenDir(path: string): void {
  const dir = path.substring(0, path.lastIndexOf("/")) || path.substring(0, path.lastIndexOf("\\"));
  if (dir) {
    localStorage.setItem(OPEN_DIR_KEY, dir);
  }
}

export function getLastSaveDir(): string | undefined {
  return localStorage.getItem(SAVE_DIR_KEY) || undefined;
}

export function setLastSaveDir(path: string): void {
  const dir = path.substring(0, path.lastIndexOf("/")) || path.substring(0, path.lastIndexOf("\\"));
  if (dir) {
    localStorage.setItem(SAVE_DIR_KEY, dir);
  }
}
