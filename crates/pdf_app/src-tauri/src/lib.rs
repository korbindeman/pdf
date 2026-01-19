use serde::Serialize;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    Emitter, Manager,
};

#[derive(Serialize)]
struct SvgDocument {
    pages: Vec<String>,
    width_pt: f64,
    height_pt: f64,
}

#[tauri::command]
fn render_markdown_to_svg(markdown: &str) -> Result<SvgDocument, String> {
    let doc = pdf_core::markdown_to_svg(markdown)?;
    Ok(SvgDocument {
        pages: doc.pages,
        width_pt: doc.width_pt,
        height_pt: doc.height_pt,
    })
}

#[tauri::command]
fn save_pdf_to_file(markdown: &str, path: &str) -> Result<(), String> {
    let pdf_bytes = pdf_core::markdown_to_pdf(markdown)?;
    std::fs::write(path, pdf_bytes).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let open_file = MenuItemBuilder::with_id("open_file", "Open File...")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;

            let save_pdf = MenuItemBuilder::with_id("save_pdf", "Save PDF...")
                .accelerator("CmdOrCtrl+R")
                .build(app)?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&open_file)
                .item(&save_pdf)
                .separator()
                .close_window()
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let app_menu = SubmenuBuilder::new(app, "PDF Editor").quit().build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_menu)
                .item(&file_menu)
                .item(&edit_menu)
                .build()?;

            app.set_menu(menu)?;

            Ok(())
        })
        .on_menu_event(|app, event| {
            if let Some(window) = app.get_webview_window("main") {
                match event.id().as_ref() {
                    "open_file" => {
                        let _ = window.emit("menu-open-file", ());
                    }
                    "save_pdf" => {
                        let _ = window.emit("menu-save-pdf", ());
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            render_markdown_to_svg,
            save_pdf_to_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
