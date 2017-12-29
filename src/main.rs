// Based on the tutorial at
//   https://mmstick.github.io/gtkrs-tutorials/chapter_01.html
extern crate gtk;
extern crate cairo;
use gtk::*;
use cairo::*;
use std::process;

mod bulbocl;
use bulbocl::*;

pub struct App {
    pub window: Window,
    pub topvbox: Box,
    pub hbox1: Box,
    pub outputis: ImageSurface,
    pub outputimage: Image,

    pub powerhbox: Box,
    pub powerlabel: Label,
    pub powerscale: Scale,

    pub bulbocl: Bulbocl
}

impl App {
    fn new() -> App {
        let window = Window::new(WindowType::Toplevel);
        window.set_title("Mandelbulb");
        window.set_wmclass("app-name", "Mandelbulb");
        //Window::set_default_icon_name("iconname");

        window.connect_delete_event(|_,_| {
            main_quit();
            Inhibit(false)
        });
        let mut bulbocl = Bulbocl::new();
        bulbocl.calc_bulb(256);

        // Inside the window, bottom is controls, top is image and
        // more controls
        let topvbox = Box::new(Orientation::Vertical, 2);
        window.add(&topvbox);
        // Inside the topvbox top section
        let hbox1 = Box::new(Orientation::Horizontal, 2);
        topvbox.pack_start(&hbox1, true, true, 0);

        // Display the output image - it's a pixbuf in an Image
        let mut outputis = ImageSurface::create(Format::Rgb24, 640, 480).unwrap();
        // TODO: Check the ImageSurface stride is what we expect with get_stride or better pass it
        // into the OCL
        //bulbocl.render_image(outputis.get_data().unwrap().as_mut_ptr(), 640, 480);
        {
            let mut id = outputis.get_data().unwrap();
            bulbocl.render_image(&mut id, 640, 480);
        }
        let outputimage = Image::new_from_surface(Some(outputis.as_ref()));
        hbox1.pack_start(&outputimage, true, true, 0);

        let powerhbox = Box::new(Orientation::Horizontal, 2);
        let powerlabel = Label::new("Power:");
        let powerscale = Scale::new_with_range( gtk::Orientation::Horizontal, 1.0, 10.0, 0.25);
        powerhbox.pack_start(&powerlabel, false, false, 0);
        powerhbox.pack_end(&powerscale, true, true, 10 /* Pad: To stop slider overlapping text */);
        topvbox.pack_end(&powerhbox, true, true, 0);

        App { window, topvbox, hbox1, outputis: outputis, outputimage,
              powerhbox, powerlabel, powerscale,
              bulbocl: bulbocl }
    }
}

fn main() {
    if gtk::init().is_err() {
        eprintln!("failed to init GTK app");
        process::exit(1);
    }

    App::new().window.show_all();

    gtk::main();
}
