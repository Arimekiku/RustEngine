use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

pub fn window_test(event_loop : EventLoop<()>) {
    event_loop.run(|event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            _ => ()
        }
    });
}