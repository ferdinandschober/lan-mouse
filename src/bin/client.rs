use std::{os::unix::prelude::RawFd, os::unix::prelude::AsRawFd};
use lan_mouse::protocol;

use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1 as VpManager,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1 as Vp,
};

use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1 as VkManager,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1 as Vk,
};

use wayland_client::{
    protocol::{wl_registry, wl_seat, wl_keyboard},
    Connection, Dispatch, EventQueue, QueueHandle,
};

// App State, implements Dispatch event handlers
struct App {
    vpm: Option<VpManager>,
    vkm: Option<VkManager>,
    seat: Option<wl_seat::WlSeat>,
    keymap: Option<(wl_keyboard::KeymapFormat, RawFd, u32)>,
}

// Implement `Dispatch<WlRegistry, ()> event handler
// user-data = ()
impl Dispatch<wl_registry::WlRegistry, ()> for App {
    fn event(
        app: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<App>,
    ) {
        // Match global event to get globals after requesting them in main
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            // println!("[{}] {} (v{})", name, interface, version);
            match &interface[..] {
                "wl_keyboard" => {
                    registry.bind::<wl_keyboard::WlKeyboard, _, _>(name, 1, qh, ());
                }
                "wl_seat" => {
                    app.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ()));
                }
                "zwlr_virtual_pointer_manager_v1" => {
                    app.vpm = Some(registry.bind::<VpManager, _, _>(name, 1, qh, ()));
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    app.vkm = Some(registry.bind::<VkManager, _, _>(name, 1, qh, ()));
                }
                _ => {}
            }
        }
    }
}

// The main function of our program
fn main() {
    let config = lan_mouse::config::Config::new("config.toml").unwrap();
    // establish connection via environment-provided configuration.
    let conn = Connection::connect_to_env().unwrap();

    // Retrieve the wayland display object
    let display = conn.display();

    // Create an event queue for our event processing
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    // Create a wl_registry object by sending the wl_display.get_registry request
    let _registry = display.get_registry(&qh, ());

    let mut app = App {
        vpm: None,
        vkm: None,
        seat: None,
        keymap: None,
    };

    // use roundtrip to process this event synchronously
    event_queue.roundtrip(&mut app).unwrap();


    let vpm = app.vpm.as_ref().unwrap();
    let vkm = app.vkm.as_ref().unwrap();
    let seat = app.seat.as_ref().unwrap();
    let pointer: Vp = vpm.create_virtual_pointer(None, &qh, ());
    let keyboard: Vk = vkm.create_virtual_keyboard(&seat, &qh, ());
    while app.keymap.is_none() {
        event_queue.blocking_dispatch(&mut app).unwrap();
    }
    let (format, fd, size) = app.keymap.unwrap();
    keyboard.keymap(u32::from(format), fd, size);
    let connection = protocol::Connection::new(config);
    udp_loop(&connection, &pointer, &keyboard, event_queue).unwrap();
    println!();
}

/// main loop handling udp packets
fn udp_loop(connection: &protocol::Connection, pointer: &Vp, keyboard: &Vk, q: EventQueue<App>) -> std::io::Result<()> {
    loop {
        if let Some(event) = connection.receive() {
            match event {
                protocol::Event::Mouse { t, x, y } => {
                    pointer.motion(t, x, y);
                }
                protocol::Event::Button { t, b, s } => {
                    pointer.button(t, b, s);
                }
                protocol::Event::Axis { t, a, v } => {
                    pointer.axis(t, a, v);
                }
                protocol::Event::Key { t, k, s } => {
                    // TODO send keymap fist
                    keyboard.key(t, k, match s {
                        wl_keyboard::KeyState::Released => 0,
                        wl_keyboard::KeyState::Pressed => 1,
                        _ => 1,
                    });
                },
                protocol::Event::KeyModifier { mods_depressed, mods_latched, mods_locked, group } => {
                    keyboard.modifiers(mods_depressed, mods_latched, mods_locked, group);
                },
            }
        }

        q.flush().unwrap();
    }
}

impl Dispatch<VpManager, ()> for App {
    fn event(
        _: &mut Self,
        _: &VpManager,
        _: <VpManager as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // nothing to do here since no events are defined for VpManager
    }
}

impl Dispatch<Vp, ()> for App {
    fn event(
        _: &mut Self,
        _: &Vp,
        _: <Vp as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // no events defined for vp either
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for App {
    fn event(
        app: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Keymap { format, fd, size } = event {
            println!("keymap received");
            app.keymap = Some((format.into_result().unwrap(), fd.as_raw_fd(), size));
        }
    }
}

impl Dispatch<VkManager, ()> for App {
    fn event(
        _: &mut Self,
        _: &VkManager,
        _: <VkManager as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        //
    }
}

impl Dispatch<Vk, ()> for App {
    fn event(
        _: &mut Self,
        _: &Vk,
        _: <Vk as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        //
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for App {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        //
    }
}
