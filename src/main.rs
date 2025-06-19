extern crate libui;
use libui::controls::*;
use libui::prelude::*;
use public_ip_address::lookup::LookupProvider;
//use std::net::{IpAddr, Ipv4Addr};

use log::{LevelFilter, error, info};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{AllocAnyThread, define_class, msg_send};
use objc2_core_location::{
    CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObjectProtocol};
use once_cell::sync::Lazy;
use oslog::OsLogger;
use std::sync::RwLock;

#[derive(Debug, Copy, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastLocation {
    pub latitude: f64,
    pub longitude: f64,
}

pub static LAST_LOCATION: Lazy<RwLock<Option<LastLocation>>> = Lazy::new(|| RwLock::new(None));

// Define the MyLocationDelegate class
define_class!(
    #[name = "MyLocationDelegate"]
    #[unsafe(super = NSObject)]
    #[thread_kind = AllocAnyThread]
    struct MyLocationDelegate;

    unsafe impl CLLocationManagerDelegate for MyLocationDelegate {
        #[unsafe(method(locationManager:didChangeAuthorizationStatus:))]
        fn did_change_authorization(
            &self,
            _manager: &CLLocationManager,
            status: CLAuthorizationStatus,
        ) {
            info!("Authorization status changed: {:?}", status);
        }

        #[unsafe(method(locationManager:didUpdateLocations:))]
        fn did_update_locations(
            &self,
            _manager: &CLLocationManager,
            locations: &NSArray<CLLocation>,
        ) {
            if let Some(loc) = locations.firstObject() {
                let coord = unsafe { loc.coordinate() };
                info!("new geolocation: {}, {}", coord.latitude, coord.longitude);
                let mut lock = LAST_LOCATION.write().unwrap();

                *lock = Some(LastLocation {
                    latitude: coord.latitude,
                    longitude: coord.longitude,
                });
            }
        }

        #[unsafe(method(locationManager:didFailWithError:))]
        fn did_fail_with_error(&self, _manager: &CLLocationManager, _error: &AnyObject) {
            error!("Location manager failed with error");
        }
    }

    unsafe impl NSObjectProtocol for MyLocationDelegate {}
);

impl MyLocationDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc();
        unsafe { msg_send![this, init] }
    }
}

// Global delegate holder
static LOCATION_DELEGATE: Lazy<Retained<MyLocationDelegate>> =
    Lazy::new(|| MyLocationDelegate::new());

// Main-thread-only CLLocationManager holder
static mut LOCATION_MANAGER: Option<Retained<CLLocationManager>> = None;

fn main() {
    OsLogger::new("rlocation")
        .level_filter(LevelFilter::Debug)
        .category_level_filter("Settings", LevelFilter::Trace)
        .init()
        .unwrap();

    let ui = UI::init().expect("Couldn't initialize UI library");

    let mut win = Window::new(&ui, "rlocation", 300, 200, WindowType::NoMenubar);
    let mut layout = VerticalBox::new();
    let mut label_text = String::new();
    let loc_label = Label::new("Unknown");

    // ask for location

    let manager = unsafe { CLLocationManager::new() };

    info!("Location manager started");

    unsafe {
        let delegate: &ProtocolObject<dyn CLLocationManagerDelegate> =
            ProtocolObject::from_ref(&**LOCATION_DELEGATE);
        manager.setDelegate(Some(delegate));
        manager.requestAlwaysAuthorization();
        manager.requestLocation();
    }

    // make a call to an IP location service
    let providers = vec![(LookupProvider::IpQuery, None)];
    let ip_result = public_ip_address::perform_lookup_with(providers, None);
    //println!("ip_result: {}", ip_result);
    let ip_display = ip_result
        .map(|response| response.to_string())
        .unwrap_or_else(|_| "Unable to get IP address".to_string());
    label_text.push_str(&format!("IP Address: {}", ip_display));
    //let label = Label::new(&label_text);
    let mut label = MultilineEntry::new();
    label.append(&label_text);

    // Update the label with the apple location data if it's available
    let mut location_text = String::new();
    if let Ok(location_guard) = LAST_LOCATION.read() {
        if let Some(location) = *location_guard {
            location_text = format!(
                "\nLatitude: {:.6}\nLongitude: {:.6}",
                location.latitude, location.longitude
            );
        } else {
            location_text = "\nLocation: Not available yet".to_string();
        }
    }

    label.append(&location_text);
    layout.append(label, LayoutStrategy::Stretchy);
    layout.append(loc_label.clone(), LayoutStrategy::Stretchy);

    win.set_child(layout);
    win.show();
    //    ui.main();
    let mut event_loop = ui.event_loop();
    event_loop.on_tick({
        let mut win = win.clone();
        let mut loc_label = loc_label.clone();
        let ui = ui.clone();
        move || {
            win.set_title("rlocation");
            let mut location_text = String::new();
            if let Ok(location_guard) = LAST_LOCATION.read() {
                if let Some(location) = *location_guard {
                    location_text = format!(
                        "\nLatitude: {:.6}\nLongitude: {:.6}",
                        location.latitude, location.longitude
                    );
                    ui.quit();
                } else {
                    location_text = "\nLocation: Not available yet".to_string();
                }
            }

            loc_label.set_text(&location_text);
        }
    });
    event_loop.run();
}
