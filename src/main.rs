extern crate libui;
use libui::controls::*;
use libui::prelude::*;
use public_ip_address::lookup::LookupProvider;
//use std::net::{IpAddr, Ipv4Addr};


fn main() {
    embed_plist::embed_info_plist!("../Info.plist");    
    let ui = UI::init()
        .expect("Couldn't initialize UI library");
    
    let mut win = Window::new(&ui, "rlocation", 300, 200, 
        WindowType::NoMenubar);
    let mut layout = VerticalBox::new();
    let mut label_text = String::new(); 

    let providers = vec![(LookupProvider::IpQuery,None)];
    let ip_result = public_ip_address::perform_lookup_with(providers,None);
    //println!("ip_result: {}", ip_result);
    let ip_display = ip_result
        .map(|response| response.to_string())
        .unwrap_or_else(|_| "Unable to get IP address".to_string());
    label_text.push_str(&format!("IP Address: {}", ip_display));
    //let label = Label::new(&label_text);
    let mut label = MultilineEntry::new();
    label.append(&label_text);

    layout.append(label, LayoutStrategy::Stretchy);    

    win.set_child(layout);
    win.show();
    ui.main();
}
