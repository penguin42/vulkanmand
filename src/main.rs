// Based on the tutorial at
//   https://mmstick.github.io/gtkrs-tutorials/chapter_01.html
extern crate glib;
extern crate gdk;
extern crate gtk;
extern crate cairo;
extern crate nalgebra as na;

use gtk::*;
use cairo::*;
use std::fs::File;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

mod bulbvulk;
use bulbvulk::*;

pub struct State {
    power: f32,
    // These vectors are in voxel space/voxelsize - i.e. 0..1 so 0.5,0.5 is over the middle
    eye: na::Vector3<f32>,
    // The eye looks towards the centre of the viewplane
    vp_mid: na::Vector3<f32>,
    // The viewplane is as big as the image, the point the eye looks towards
    // is calculated by adding fractions of the right and down vectors
    vp_right: na::Vector3<f32>,
    vp_down: na::Vector3<f32>,

    light: na::Vector3<f32>
}

impl State {
    fn new() -> State {
        State { power: 8.0,
                eye: na::Vector3::new(0.5, 0.5, -2.0),
                vp_mid: na::Vector3::new(0.5, 0.5, -0.75),
                vp_right: na::Vector3::new(0.3, 0.0, 0.0),
                vp_down: na::Vector3::new(0.0, 0.3, 0.0),
                light: na::Vector3::new(0.3, -0.5, -0.5)
        }
    }
}

pub struct App {
    pub window: Window,
    pub outputis: ImageSurface,

    pub rotxbutminus: Button,
    pub rotxbutplus: Button,
    pub rotybutminus: Button,
    pub rotybutplus: Button,
    pub rotzbutminus: Button,
    pub rotzbutplus: Button,

    pub zoomin: Button,
    pub zoomout: Button,

    pub saveimagebut: Button,
    pub savevoxelsbut: Button,

    pub statsfullval: Label,
    pub statstraceval: Label,

    pub powerscale: Scale,

    pub bulbvulk: Bulbvulk,
    pub state: State,
}

impl App {
    fn new(state: State) -> App {
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
        //let win_id = win.get_id();
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

        let zoomhbox = Box::new(Orientation::Horizontal, 3);
        let zoomin = Button::new_from_icon_name("zoom-in", IconSize::Button.into());
        let zoomout = Button::new_from_icon_name("zoom-out", IconSize::Button.into());
        zoomhbox.pack_start(&Label::new("Zoom:"), false, false, 0);
        zoomhbox.pack_start(&zoomin, false, false, 0);
        zoomhbox.pack_start(&zoomout, false, false, 0);
        topcontvbox.pack_start(&zoomhbox, false, false, 0);

        // Buttons for saving stuff out
        let savehbox = Box::new(Orientation::Horizontal, 3);
        let saveimagebut = Button::new_with_label("image");
        let savevoxelsbut = Button::new_with_label("voxels");
        savehbox.pack_start(&Label::new("Save:"), false, false, 0);
        savehbox.pack_start(&saveimagebut, false, false, 0);
        savehbox.pack_start(&savevoxelsbut, false, false, 0);
        topcontvbox.pack_end(&savehbox, false, false, 0);

        // Stats
        let statsfullhbox = Box::new(Orientation::Horizontal, 2);
        let statsfullval   = Label::new("---.---");
        let statstracehbox = Box::new(Orientation::Horizontal, 2);
        let statstraceval   = Label::new("---.---");
        statsfullhbox.pack_start(&Label::new("Recalc (ms):"), true, true, 0);
        statsfullhbox.pack_end(&statsfullval, true, true, 0);
        statstracehbox.pack_start(&Label::new("Rerender (ms)"), true, true, 0);
        statstracehbox.pack_end(&statstraceval, true, true, 0);
        topcontvbox.pack_end(&statsfullhbox, false, false, 0);
        topcontvbox.pack_end(&statstracehbox, false, false, 0);
        hbox1.pack_end(&topcontvbox, false, false, 0);

        let powerhbox = Box::new(Orientation::Horizontal, 2);
        let powerlabel = Label::new("Power:");
        let powerscale = Scale::new_with_range( gtk::Orientation::Horizontal, 1.0, 10.0, 0.25);
        powerscale.set_value(8.0);
        powerhbox.pack_start(&powerlabel, false, false, 0);
        powerhbox.pack_end(&powerscale, true, true, 10 /* Pad: To stop slider overlapping text */);
        topvbox.pack_end(&powerhbox, true, true, 0);

        window.show_all();
        let bulbvulk = Bulbvulk::new(outputimage);

        App { window, outputis: outputis, powerscale,
              rotxbutplus, rotxbutminus,
              rotybutplus, rotybutminus,
              rotzbutplus, rotzbutminus,
              zoomin, zoomout,
              saveimagebut, savevoxelsbut,
              statsfullval, statstraceval, bulbvulk, state
            }
    }

    fn init(mut self)
    {
        do_redraw(&mut self, true);

        let apprc : Rc<RefCell<App>> = Rc::new(RefCell::new(self));
        let appb = apprc.borrow();
        {
            let powerscale_adjust = appb.powerscale.get_adjustment();
            let app = apprc.clone();

            powerscale_adjust.connect_value_changed(move |adj| {
                app.borrow_mut().state.power = adj.get_value() as f32;
                do_redraw(&mut app.borrow_mut(), true);
            });
        }
        {
            let app = apprc.clone();

            appb.rotxbutminus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), -1.0, 0.0, 0.0); });
        }
        {
            let app = apprc.clone();

            appb.rotxbutplus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), 1.0, 0.0, 0.0); });
        }
        {
            let app = apprc.clone();

            appb.rotybutminus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), 0.0, -1.0, 0.0); });
        }
        {
            let app = apprc.clone();

            appb.rotybutplus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), 0.0, 1.0, 0.0); });
        }
        {
            let app = apprc.clone();

            appb.rotzbutminus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), 0.0, 0.0, -1.0); });
        }
        {
            let app = apprc.clone();

            appb.rotzbutplus.connect_clicked(move |_| { do_rotate(&mut app.borrow_mut(), 0.0, 0.0, 1.0); });
        }
        {
            let app = apprc.clone();

            appb.zoomin.connect_clicked(move |_| { do_zoom(&mut app.borrow_mut(), 1.0/1.2); });
        }
        {
            let app = apprc.clone();

            appb.zoomout.connect_clicked(move |_| { do_zoom(&mut app.borrow_mut(), 1.2); });
        }
        {
            let app = apprc.clone();

            appb.saveimagebut.connect_clicked(move |_| { app.borrow_mut().save_image(); });
        }
        {
            let app = apprc.clone();

            appb.savevoxelsbut.connect_clicked(move |_| { app.borrow_mut().bulbvulk.save_voxels(); });
        }
    }

    fn save_image(&self) {
        let mut file = File::create("image.png").unwrap();
        self.outputis.write_to_png(&mut file).unwrap();
    }
}

fn do_redraw(app: &mut App, recalc_fractal: bool) {
    let start = Instant::now();

    if recalc_fractal {
        app.bulbvulk.calc_bulb(384, app.state.power);
    }
    {
        app.bulbvulk.render_image(512, 512, app.state.eye, app.state.vp_mid, app.state.vp_right, app.state.vp_down, app.state.light );
    }

    let end = Instant::now();
    let duration = end.duration_since(start);
    let durationms = duration.as_secs() as f32 / 1000.0 + duration.subsec_nanos() as f32 / 1000000.0;
    let durationstr = format!("{:.*}", 3, durationms);
    if recalc_fractal {
        app.statsfullval.set_text(&durationstr);
    } else {
        app.statstraceval.set_text(&durationstr);
    }
}

fn do_rotate(app: &mut App, x: f32, y: f32, z: f32) {
    let x = x*std::f32::consts::PI / 10.0;
    let y = y*std::f32::consts::PI / 10.0;
    let z = z*std::f32::consts::PI / 10.0;
    // The centre point of the mandelbulb is 0.5/0.5/0.5 - so translate down to 0, rotate and
    // translate back (Is there an easier way in nalgebra's Rotation3?)
    let offset = na::Vector3::new(0.5, 0.5, 0.5);
    let rot = na::Rotation3::from_euler_angles(x,y,z); // order???
    // eye and vp_mid are points in space so need the translations
    app.state.eye = offset + rot * (app.state.eye - offset);
    app.state.vp_mid = offset + rot * (app.state.vp_mid - offset);
    app.state.light = offset + rot * (app.state.light - offset);
    // vp_right/vp_down are relative vectors so dont need the translations
    app.state.vp_right = rot * app.state.vp_right;
    app.state.vp_down = rot * app.state.vp_down;
    do_redraw(app, false);
}

fn do_zoom(app: &mut App, scale: f32) {
    app.state.vp_right *= scale;
    app.state.vp_down *= scale;
    do_redraw(app, false);
}
fn main() -> Result<(), glib::error::BoolError> {
    gtk::init()?;

    App::new(State::new()).init();

    gtk::main();

    Ok(())
}
