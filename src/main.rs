// Based on the tutorial at
//   https://mmstick.github.io/gtkrs-tutorials/chapter_01.html
extern crate gtk;
extern crate cairo;
use gtk::*;
use cairo::*;
use std::process;
use std::cell::RefCell;
use std::rc::Rc;

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
}

struct State {
    power: f32
}

impl App {
    fn new(bulbocl: &mut Bulbocl) -> App {
        let window = Window::new(WindowType::Toplevel);
        window.set_title("Mandelbulb");
        window.set_wmclass("app-name", "Mandelbulb");
        //Window::set_default_icon_name("iconname");

        window.connect_delete_event(|_,_| {
            main_quit();
            Inhibit(false)
        });
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
        {
            let mut id = outputis.get_data().unwrap();
            bulbocl.render_image(&mut id, 640, 480);
        }

        let outputimage = Image::new_from_surface(Some(outputis.as_ref()));
        hbox1.pack_start(&outputimage, true, true, 0);

        let powerhbox = Box::new(Orientation::Horizontal, 2);
        let powerlabel = Label::new("Power:");
        let powerscale = Scale::new_with_range( gtk::Orientation::Horizontal, 1.0, 10.0, 0.25);
        powerscale.set_value(8.0);
        powerhbox.pack_start(&powerlabel, false, false, 0);
        powerhbox.pack_end(&powerscale, true, true, 10 /* Pad: To stop slider overlapping text */);
        topvbox.pack_end(&powerhbox, true, true, 0);

        App { window, topvbox, hbox1, outputis: outputis, outputimage,
              powerhbox, powerlabel, powerscale,
            }
    }

}

fn do_redraw(app: &mut App, bulbocl: &mut Bulbocl, state: &mut State, recalc_fractal: bool) {
    if recalc_fractal {
        bulbocl.calc_bulb(256, state.power);
    }
    app.outputimage.set_from_surface(None);
    {
        let mut id = app.outputis.get_data().unwrap();
        bulbocl.render_image(&mut id, 640, 480);
    }
    app.outputimage.set_from_surface(Some(app.outputis.as_ref()));
}

fn wire_callbacks(app: Rc<RefCell<App>>, bulbocl: Rc<RefCell<Bulbocl>>, state: Rc<RefCell<State>>)
{
    let powerscale_adjust = app.borrow().powerscale.get_adjustment();
    powerscale_adjust.connect_value_changed(move |adj| {
        state.borrow_mut().power = adj.get_value() as f32;
        do_redraw(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), true);
    });
}

fn main() {
    let mut bulbocl = Bulbocl::new();
    bulbocl.calc_bulb(256, 8.0);

    if gtk::init().is_err() {
        eprintln!("failed to init GTK app");
        process::exit(1);
    }
    let apprc : Rc<RefCell<App>> = Rc::new(RefCell::new(App::new(&mut bulbocl)));
    let staterc : Rc<RefCell<State>> = Rc::new(RefCell::new(State { power: 8.0 }));
    let bulboclrc : Rc<RefCell<Bulbocl>> = Rc::new(RefCell::new(bulbocl));

    apprc.borrow().window.show_all();
    wire_callbacks(apprc.clone(), bulboclrc.clone(), staterc.clone());

    gtk::main();
}
