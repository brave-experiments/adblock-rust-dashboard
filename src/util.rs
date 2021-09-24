/// Triggers the browser to download a binary file with the given name and content.
pub fn save_bin_file(name: &str, data: &[u8]) {
    use wasm_bindgen::JsCast;
    let dat_data_url = format!("data:application/octet-stream;base64,{}", base64::encode(data));
    let document = web_sys::window().unwrap().document().unwrap();

    let download_link: web_sys::HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
    download_link.set_download(name);
    download_link.set_href(&dat_data_url);

    let event = document.create_event("MouseEvents").unwrap();
    event.init_event_with_bubbles_and_cancelable("click", true, true);
    download_link.dispatch_event(&event).unwrap();
}
