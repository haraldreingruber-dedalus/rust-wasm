use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::marker::{Send};
use std::ffi::{CString};
use std::sync::Mutex;
use std::any::Any;
use std::fs::File;
use std::io::prelude::*;
use sdl2::video::{Window, WindowContext};
use sdl2::image::{LoadSurface};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::surface::Surface;
use sdl2::event::Event;
use sdl2::pixels::{Color};
use sdl2::rect::{Rect, Point};
use stdweb::web::TypedArray;

use config::{Config};
use utils::{self, SizedTexture};
use actions::Action;
use gesture::{GestureDetector, GestureEvent, GestureDetectorTypes};

static mut TEXTURE_CREATOR: Option<TextureCreator<WindowContext>> = None;
lazy_static!{
    static ref LOAD_REGISTER: Mutex<HashMap<String, SizedTexture>> = Mutex::new(HashMap::new());
    static ref LOADING_IMGS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

pub trait Display {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect);
    fn handle_events(&mut self, event: &Event) -> Option<Action> { None }
    fn is_interactive(&self) -> bool { false }
    fn update(&mut self) {}
    fn on_start(&mut self, &Option<Box<Any>>) {}
}

pub struct Stage {
    children: HashMap<String, Rc<RefCell<Display>>>,
    active_scene: Option<Rc<RefCell<Display>>>,
    params: Option<Box<Any>>,
}

impl Stage {
    pub fn new(tc: TextureCreator<WindowContext>) -> Rc<RefCell<Stage>> {
        unsafe {
            TEXTURE_CREATOR = Some(tc);
        }
        Rc::new(RefCell::new(Stage {
            children: HashMap::new(),
            active_scene: None,
            params: None,
        }))
    }
    pub fn add_scene(&mut self, key: &str, c: Rc<RefCell<Display>>) {
        self.children.insert(key.to_owned(), c);
    }
    pub fn start(&mut self, key: &str) {
        if let Some(s) = self.children.get(key) {
            let s = s.clone();
            {
                let params = self.get_params();
                s.borrow_mut().on_start(params);
            }
            self.active_scene = Some(s);
        }
    }
    pub fn update(&self) {
        if let Some(ref scene) = self.active_scene {
            scene.borrow_mut().update();
        }
    }
    pub fn set_params(&mut self, p: Box<Any>) {
        self.params = Some(p);
    }
    pub fn get_params(&self) -> &Option<Box<Any>> {
        &self.params
    }
}

impl Display for Stage {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        if let Some(ref scene) = self.active_scene {
            scene.borrow().render(canvas, rect.clone());
        }
    }
    fn handle_events(&mut self, event: &Event) -> Option<Action> {
        let mut action = None;
        if let Some(ref scene) = self.active_scene {
            action = scene.borrow_mut().handle_events(&event);
        }
        if let Some(a) = action {
            match a {
                Action::ShowPreview(i) => {
                    self.set_params(Box::new(i));
                    self.start("preview");
                },
                Action::ShowGallery => {
                    self.start("gallery");
                },
                _ => (),
            }
        }
        None
    }
}

pub enum FillMode {
    Cover,
    Contain,
}

static mut DEFAULT_LOADED: bool = false;
const DEFAULT_IMG: &'static str = "../assets/iconmonstr-picture-1-240.png";
/// image from network are not loaded when you call load
/// image from localdisk are loaded eagerly
pub struct Image {
    dirty: bool,
    src: String,
    w: u32,
    h: u32,
    fill: FillMode,
    local: bool,
}

impl Image {
    pub fn new(src: String) -> Rc<RefCell<Image>> {
        Rc::new(RefCell::new(Image {
            dirty: false,
            src,
            ..Default::default()
        }))
    }
    pub fn new_with_dimension_local(src: String, w: u32, h: u32) -> Image {
        if src != "" {
            load_local_img(&src);
        }
        Image {
            dirty: false,
            src,
            w,
            h,
            local: true,
            ..Default::default()
        }
    }
    pub fn new_with_dimension(src: String, w: u32, h: u32) -> Rc<RefCell<Image>> {
        Rc::new(RefCell::new(Image {
            dirty: false,
            src,
            w,
            h,
            ..Default::default()
        }))
    }
    pub fn load(&self) {
        if self.src == "" {
            return;
        }
        // load default image is not loaded
        unsafe {
            if !DEFAULT_LOADED {
                load_local_img(DEFAULT_IMG);
                DEFAULT_LOADED = true;
            }
        }
        if self.local {
            load_local_img(&self.src);
        } else {
            load_img(&self.src);
        }
    }
    pub fn is_loaded(&self) -> bool {
        if self.local {
            return true;
        }
        let m = LOAD_REGISTER.lock().unwrap();
        m.get(&self.src).is_some()
    }
    pub fn get_src(&mut self) -> &str {
        &self.src
    }
    pub fn set_src(&mut self, src: &str) {
        self.src = src.to_string();
        if self.local {
            self.load();
        }
    }
    pub fn set_fill(&mut self, v: FillMode) {
        self.fill = v;
    }
    pub fn get_img_size(&self) -> Option<(u32, u32)> {
        let m = LOAD_REGISTER.lock().unwrap();
        if let Some(&SizedTexture(img_w, img_h, ..)) = m.get(&self.src) {
            Some((img_w, img_h))
        } else {
            None
        }
    }

    pub fn cover_size(img_w: u32, img_h: u32, w: u32, h: u32) -> (u32, u32) {
        let img_r = img_w as f64 / img_h as f64;
        let r = w as f64 / h as f64;
        if img_r > r {
            ((h as f64 * img_r) as u32, h)
        } else {
            (w, (w as f64 / img_r) as u32)
        }
    }

    pub fn contain_size(img_w: u32, img_h: u32, w: u32, h: u32) -> (u32, u32) {
        let img_r = img_w as f64 / img_h as f64;
        let r = w as f64 / h as f64;
        if img_r > r {
            (w, (w as f64 / img_r) as u32)
        } else {
            ((img_r * h as f64) as u32, h)
        }
    }
}

impl Display for Image {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        if self.src == "" {
            return;
        }
        let m = LOAD_REGISTER.lock().unwrap();
        let prefix = if self.local { LOCAL_IMG_PREFIX } else { "" };
        let src = prefix.to_owned() + &self.src;
        let texture = m.get(&src).or_else(|| m.get(&(LOCAL_IMG_PREFIX.to_owned() + DEFAULT_IMG)));
        if let Some(&SizedTexture(img_w, img_h, ref tex)) = texture {
            let s_rect = Rect::new(0, 0, img_w, img_h);

            // work out render size
            let (w, h) = match self.fill {
                FillMode::Contain => {
                    Self::contain_size(img_w, img_h, rect.width(), rect.height())
                },
                FillMode::Cover => {
                    Self::cover_size(img_w, img_h, rect.width(), rect.height())
                }
            };

            let t_rect = Rect::new((rect.width() as i32 - w as i32) / 2 + rect.x(),
                                   (rect.height() as i32 - h as i32) / 2 + rect.y(),
                                   w, h);

            canvas.set_clip_rect(rect);
            let _ = canvas.copy(tex,
                                s_rect,
                                t_rect);
            canvas.set_clip_rect(None);
        }
    }
}

impl Default for Image {
    fn default() -> Self {
        Self {
            dirty: false,
            src: "".to_string(),
            w: 0,
            h: 0,
            fill: FillMode::Contain,
            local: false,
        }
    }
}

pub fn load_img(src: &str) {
    if src == "" {
        return;
    }
    if LOADING_IMGS.lock().unwrap().contains(src) {
        return;
    }
    // check if already loaded
    let m = LOAD_REGISTER.lock().unwrap();
    if m.get(src).is_some() {
        return;
    }

    LOADING_IMGS.lock().unwrap().insert(src.to_owned());
    let src2 = src.to_owned();
    let src3 = src.to_owned();
    utils::fetch(src, move |file| {
        loaded(&src2, &file)
    }, move || {
        load_err(&src3);
    });
}

const LOCAL_IMG_PREFIX: &'static str = "!local:";
pub fn load_local_img(file: &str) {
    let src = LOCAL_IMG_PREFIX.to_owned() + file;
    // check if already loaded
    let m = LOAD_REGISTER.lock().unwrap();
    if m.get(&src).is_some() {
        return;
    }

    loaded(&src, file);
}

pub struct Button {
    rect: Rect,
    active_img: Option<Image>,
    active_color: Option<Color>,
    img: Option<Image>,
    color: Option<Color>,
    gesture_detector: GestureDetector,
}

impl Button {
    pub fn new(rect: Rect) -> Button {
        Button {
            rect,
            active_color: None,
            active_img: None,
            img: None,
            color: None,
            gesture_detector: GestureDetector::new(vec![GestureDetectorTypes::Tap]),
        }
    }
    pub fn set_img(&mut self, img: Image) {
        self.img = Some(img);
    }
}

impl Display for Button {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        if let Some(ref img) = self.img {
            img.render(canvas, self.rect);
        }
    }
    fn handle_events(&mut self, evt: &Event) -> Option<Action> {
        self.gesture_detector.feed(evt);

        // single touch
        for ref event in self.gesture_detector.poll() {
            match event {
                &GestureEvent::Tap(x, y) => {
                    let x = x * (*Config::get_u32("width").unwrap()) as f32;
                    let y = y * (*Config::get_u32("height").unwrap()) as f32;

                    if self.rect.contains_point(Point::new(x as i32, y as i32)) {
                        return Some(Action::ShowGallery);
                    }
                },
                _ => (),
            }
        }
        None
    }
    fn is_interactive(&self) -> bool {
        true
    }
}

fn loaded(src: &str, file: &str) {
    unsafe {
        let mut m = LOAD_REGISTER.lock().unwrap();
        LOADING_IMGS.lock().unwrap().remove(src);

        if let Ok(surf) = Surface::from_file(file) {
            if let Some(ref tc) = TEXTURE_CREATOR {
                let w = surf.width();
                let h = surf.height();
                let tex = tc.create_texture_from_surface(surf).expect("failed to create texture fron surface");
                m.insert(src.to_owned(), SizedTexture(w, h, tex));
            } else {
                println!("load err");
            }
        } else {
            println!("not image");
        }
    }
}

fn load_err(src: &str) {
    unsafe {
        LOADING_IMGS.lock().unwrap().remove(src);
        println!("load failed! src: {}", src);
    }
}
