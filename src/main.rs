// Based on the tutorial at
//   https://mmstick.github.io/gtkrs-tutorials/chapter_01.html
extern crate gtk;
extern crate cairo;
extern crate nalgebra as na;
use gtk::*;
use cairo::*;
use std::fs::File;
use std::process;
use std::cell::RefCell;
use std::rc::Rc;

mod bulbocl;
use bulbocl::*;

pub struct App {
    pub window: Window,
    pub outputis: ImageSurface,
    pub outputimage: Image,

    pub rotxbutminus: Button,
    pub rotxbutplus: Button,
    pub rotybutminus: Button,
    pub rotybutplus: Button,
    pub rotzbutminus: Button,
    pub rotzbutplus: Button,

    pub saveimagebut: Button,
    pub savevoxelsbut: Button,
    pub savedebugbut: Button,

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
        let outputis = ImageSurface::create(Format::Rgb24, 512, 512).unwrap();

        let outputimage = Image::new_from_surface(Some(outputis.as_ref()));
        hbox1.pack_start(&outputimage, true, true, 0);

        // Set of controls to the right of the image
        let topcontvbox  = Box::new(Orientation::Vertical, 2);
        // Set of rotation controls
        //  TODO: Replace by some type of click/drag spaceball thing
        let rotxhbox = Box::new(Orientation::Horizontal, 3);
        let rotxlabel = Label::new("Rotate X axis:");
        let rotxbutminus = Button::new_from_icon_name("go-up", IconSize::Button.into());
        let rotxbutplus = Button::new_from_icon_name("go-down", IconSize::Button.into());
        rotxhbox.pack_start(&rotxlabel, false, false, 0);
        rotxhbox.pack_start(&rotxbutminus, false, false, 0);
        rotxhbox.pack_start(&rotxbutplus, false, false, 0);
        topcontvbox.pack_start(&rotxhbox, false, false, 0);
        let rotyhbox = Box::new(Orientation::Horizontal, 3);
        let rotylabel = Label::new("Rotate Y axis:");
        let rotybutminus = Button::new_from_icon_name("go-previous", IconSize::Button.into());
        let rotybutplus = Button::new_from_icon_name("go-next", IconSize::Button.into());
        rotyhbox.pack_start(&rotylabel, false, false, 0);
        rotyhbox.pack_start(&rotybutminus, false, false, 0);
        rotyhbox.pack_start(&rotybutplus, false, false, 0);
        topcontvbox.pack_start(&rotyhbox, false, false, 0);
        let rotzhbox = Box::new(Orientation::Horizontal, 3);
        let rotzlabel = Label::new("Rotate Z axis:");
        // Todo using named icons seems to be a bad idea, these rotate ones look nothing like the
        // go's
        let rotzbutminus = Button::new_from_icon_name("object-rotate-left", IconSize::Button.into());
        let rotzbutplus = Button::new_from_icon_name("object-rotate-right", IconSize::Button.into());
        rotzhbox.pack_start(&rotzlabel, false, false, 0);
        rotzhbox.pack_start(&rotzbutminus, false, false, 0);
        rotzhbox.pack_start(&rotzbutplus, false, false, 0);
        topcontvbox.pack_start(&rotzhbox, false, false, 0);

        // Buttons for saving stuff out
        let savehbox = Box::new(Orientation::Horizontal, 3);
        let saveimagebut = Button::new_with_label("image");
        let savevoxelsbut = Button::new_with_label("voxels");
        let savedebugbut = Button::new_with_label("debug");
        savehbox.pack_start(&Label::new("Save:"), false, false, 0);
        savehbox.pack_start(&saveimagebut, false, false, 0);
        savehbox.pack_start(&savevoxelsbut, false, false, 0);
        savehbox.pack_start(&savedebugbut, false, false, 0);
        topcontvbox.pack_end(&savehbox, false, false, 0);
        hbox1.pack_end(&topcontvbox, false, false, 0);

        let powerhbox = Box::new(Orientation::Horizontal, 2);
        let powerlabel = Label::new("Power:");
        let powerscale = Scale::new_with_range( gtk::Orientation::Horizontal, 1.0, 10.0, 0.25);
        powerscale.set_value(8.0);
        powerhbox.pack_start(&powerlabel, false, false, 0);
        powerhbox.pack_end(&powerscale, true, true, 10 /* Pad: To stop slider overlapping text */);
        topvbox.pack_end(&powerhbox, true, true, 0);

        App { window, outputis: outputis, outputimage, powerscale,
              rotxbutplus, rotxbutminus,
              rotybutplus, rotybutminus,
              rotzbutplus, rotzbutminus,
              saveimagebut, savevoxelsbut, savedebugbut
            }
    }

    fn save_image(&self) {
        let mut file = File::create("image.png").unwrap();
        self.outputis.write_to_png(&mut file).unwrap();
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
                //eye: na::Vector3::new(0.5, -3.0, 0.5),
                //vp_mid: na::Vector3::new(0.5, -2.0, 0.5),
                //vp_right: na::Vector3::new(1.0, 0.0, 0.0),
                //vp_down: na::Vector3::new(0.0, 0.0, 1.0)
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
        bulbocl.render_image(&mut id, 512, 512, state.eye, state.vp_mid, state.vp_right, state.vp_down );
    }
    app.outputimage.set_from_surface(Some(app.outputis.as_ref()));
}

fn do_rotate(app: &mut App, bulbocl: &mut Bulbocl, state: &mut State, x: f32, y: f32, z: f32) {
    let x = x*std::f32::consts::PI / 10.0;
    let y = y*std::f32::consts::PI / 10.0;
    let z = z*std::f32::consts::PI / 10.0;
    println!("do_rotate {} {} {} from eye={} vp mid/r/d= {}/{}/{}", x, y, z, state.eye, state.vp_mid, state.vp_right, state.vp_down);
    // The centre point of the mandelbulb is 0.5/0.5/0.5 - so translate down to 0, rotate and
    // translate back (Is there an easier way in nalgebra's Rotation3?)
    let offset = na::Vector3::new(0.5, 0.5, 0.5);
    let rot = na::Rotation3::from_euler_angles(x,y,z); // order???
    // eye and vp_mid are points in space so need the translations
    state.eye = offset + rot * (state.eye - offset);
    state.vp_mid = offset + rot * (state.vp_mid - offset);
    // vp_right/vp_down are relative vectors so dont need the translations
    state.vp_right = rot * state.vp_right;
    state.vp_down = rot * state.vp_down;
    println!("do_rotate rot={} giving: from eye={} vp mid/r/d= {}/{}/{}", rot, state.eye, state.vp_mid, state.vp_right, state.vp_down);
    do_redraw(app, bulbocl, state, true);
}

fn wire_callbacks(app: Rc<RefCell<App>>, bulbocl: Rc<RefCell<Bulbocl>>, state: Rc<RefCell<State>>)
{
    {
        let powerscale_adjust = app.borrow().powerscale.get_adjustment();
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        powerscale_adjust.connect_value_changed(move |adj| {
            state.borrow_mut().power = adj.get_value() as f32;
            do_redraw(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), true);
        });
    }
    {
        let button = &app.borrow().rotxbutminus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), -1.0, 0.0, 0.0); });
    }
    {
        let button = &app.borrow().rotxbutplus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), 1.0, 0.0, 0.0); });
    }
    {
        let button = &app.borrow().rotybutminus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), 0.0, -1.0, 0.0); });
    }
    {
        let button = &app.borrow().rotybutplus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), 0.0, 1.0, 0.0); });
    }
    {
        let button = &app.borrow().rotzbutminus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), 0.0, 0.0, -1.0); });
    }
    {
        let button = &app.borrow().rotzbutplus;
        let app = app.clone(); let bulbocl = bulbocl.clone(); let state = state.clone();

        button.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), &mut bulbocl.borrow_mut(), &mut state.borrow_mut(), 0.0, 0.0, 1.0); });
    }
    {
        let button = &app.borrow().saveimagebut;
        let app = app.clone();

        button.connect_clicked(move |_| { app.borrow_mut().save_image(); });
    }
    {
        let button = &app.borrow().savevoxelsbut;
        let bulbocl = bulbocl.clone();

        button.connect_clicked(move |_| { bulbocl.borrow_mut().save_voxels(); });
    }
    {
        let button = &app.borrow().savedebugbut;
        let bulbocl = bulbocl.clone();

        button.connect_clicked(move |_| { bulbocl.borrow_mut().save_debug(); });
    }
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
