// Based on the tutorial at
//   https://mmstick.github.io/gtkrs-tutorials/chapter_01.html
extern crate gtk;
extern crate cairo;
extern crate nalgebra as na;
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
        // Inside the window, bottom is controls, top is image and
        // more controls
        let topvbox = Box::new(Orientation::Vertical, 2);
        window.add(&topvbox);
        // Inside the topvbox top section
        let hbox1 = Box::new(Orientation::Horizontal, 2);
        topvbox.pack_start(&hbox1, true, true, 0);

        // Display the output image - it's a pixbuf in an Image
        let outputis = ImageSurface::create(Format::Rgb24, 640, 480).unwrap();

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

struct State {
    power: f32,
    // These vectors are in voxel space/voxelsize - i.e. 0..1 so 0.5,0.5 is over the middle
    eye: na::Vector3<f32>,
    // The eye looks towards the centre of the viewplane
    vp_mid: na::Vector3<f32>,
    // The viewplane is as big as the image, the point the eye looks towards
    // is calculated by adding fractions of the right and down vectors
    vp_right: na::Vector3<f32>,
    vp_down: na::Vector3<f32>
}

impl State {
    fn new() -> State {
        State { power: 8.0,
                eye: na::Vector3::new(0.5, 0.5, -3.0),
                vp_mid: na::Vector3::new(0.5, 0.5, -2.0),
                vp_right: na::Vector3::new(1.0, 0.0, 0.0),
                vp_down: na::Vector3::new(0.0, 1.0, 0.0)
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
        bulbocl.render_image(&mut id, 640, 480, state.eye, state.vp_mid, state.vp_right, state.vp_down );
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
    let bulbocl = Bulbocl::new();

    if gtk::init().is_err() {
        eprintln!("failed to init GTK app");
        process::exit(1);
    }
    let apprc : Rc<RefCell<App>> = Rc::new(RefCell::new(App::new()));
    let staterc : Rc<RefCell<State>> = Rc::new(RefCell::new(State::new()));
    let bulboclrc : Rc<RefCell<Bulbocl>> = Rc::new(RefCell::new(bulbocl));

    do_redraw(&mut apprc.borrow_mut(), &mut bulboclrc.borrow_mut(), &mut staterc.borrow_mut(), true);
    apprc.borrow().window.show_all();
    wire_callbacks(apprc.clone(), bulboclrc.clone(), staterc.clone());

    gtk::main();
}
