use cacao::appkit::window::Window;
use cacao::appkit::{App, AppDelegate};
use log::{LevelFilter, error, info};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
use objc2::{AllocAnyThread, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_core_location::{
    CLAuthorizationStatus, CLLocation, CLLocationManager, CLLocationManagerDelegate,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSObjectProtocol};
use once_cell::sync::Lazy;
use oslog::OsLogger;
use public_ip_address::lookup::LookupProvider;
use std::sync::RwLock;
use std::{thread, time::Duration};

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
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            unsafe { app.terminate(None) };
        }

        #[unsafe(method(locationManager:didFailWithError:))]
        fn did_fail_with_error(&self, _manager: &CLLocationManager, _error: &AnyObject) {
            error!("Location manager failed with error, terminating");
            let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
            unsafe { app.terminate(None) };
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

#[derive(Default)]
struct BasicApp {
    window: Window,
}

impl AppDelegate for BasicApp {
    fn did_finish_launching(&self) {
        App::activate();

        self.window.set_minimum_content_size(400., 400.);
        self.window.set_title("rlocation");

        // hide app from displaying in the dock
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    }
    fn should_terminate_after_last_window_closed(&self) -> bool {
        true
    }
}

fn main() {
    OsLogger::new("rlocation")
        .level_filter(LevelFilter::Debug)
        .category_level_filter("Settings", LevelFilter::Trace)
        .init()
        .unwrap();

    // Set up location manager
    let manager = unsafe { CLLocationManager::new() };
    unsafe {
        let delegate: &ProtocolObject<dyn CLLocationManagerDelegate> =
            ProtocolObject::from_ref(&**LOCATION_DELEGATE);
        manager.setDelegate(Some(delegate));
        manager.requestAlwaysAuthorization();
        manager.requestLocation();
    }

    // Get IP address
    // let providers = vec![(LookupProvider::IpQuery, None)];
    // let ip_result = public_ip_address::perform_lookup_with(providers, None);
    // let ip_display = ip_result
    //     .map(|response| response.to_string())
    //     .unwrap_or_else(|_| "Unable to get IP address".to_string());

    // info!("IP Address: {}", ip_display);

    // Start a timer thread to terminate the app after 30 seconds
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(30));
        info!("30 second timeout reached, terminating app");
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        unsafe { app.terminate(None) };
    });

    // let app and delegates run to get location and terminate
    App::new("com.test.window", BasicApp::default()).run();
}
