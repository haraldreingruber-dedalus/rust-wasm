// TODO this can be dropped
// translate_x to f32

use display::{Image, Scene, Display, FillMode};
use model::Gallery;
use sdl2::video::{Window, WindowContext};
use sdl2::render::{Canvas, TextureCreator};
use sdl2::rect::{Rect, Point};
use sdl2::event::Event;
use sdl2::touch::num_touch_fingers;
use std::rc::{Rc, Weak};
use std::cell::{RefCell};
use std::time::{Duration};
use std::mem::drop;
use transition::Transition;
use gesture::{GestureDetector, GestureEvent};
use utils::mean::Mean;
use config::{Config};

const THUMB_W: u32 = 100;
const THUMB_H: u32 = 100;
const THUMB_GAP: u32 = 10;

pub struct GalleryView {
    parent: Weak<RefCell<Display>>,
    images: Vec<Rc<RefCell<Image>>>,
    dragging: bool,
    translate_y: i32,
    translate_y_pre: i32,
    gesture_detector: GestureDetector,
}

impl GalleryView {
    pub fn new(parent: Rc<RefCell<Scene>>) -> Rc<RefCell<GalleryView>> {
        let config = Config::get_gallery().unwrap();
        let width = *Config::get_u32("width").unwrap();
        let height = *Config::get_u32("height").unwrap();
        let images = config.pics.iter().map(|ref p| {
            let img = Image::new_with_dimension(p.preview.to_owned(), THUMB_W, THUMB_H);
            img.borrow_mut().set_fill(FillMode::COVER);
            img
        }).collect();

        let mut g = GalleryView {
            parent: Rc::downgrade(&parent),
            images,
            dragging: false,
            translate_y: 0,
            translate_y_pre: 0,
            gesture_detector: GestureDetector::new(),
        };
        Rc::new(RefCell::new(g))
    }
    fn image_under_point(&self, x: u32, y: u32) -> Option<usize> {
        let (n, w, h) = self.get_row_layout();
        for i in 0..self.images.len() {
            let (rx, ry) = self.item_center(n, w, h, i);
            let r = Rect::from_center(Point::new(rx as i32, ry as i32), w, h);
            if r.contains_point(Point::new(x as i32, y as i32)) {
                return Some(i)
            }
        }
        None
    }
    fn get_row_layout(&self) -> (u32, u32, u32) {
        let width = *Config::get_u32("width").unwrap();

        // n items each row
        let n = ((width as f32 - THUMB_GAP as f32) / (THUMB_GAP + THUMB_W) as f32).floor() as u32;
        let w = (width - THUMB_GAP) / n - THUMB_GAP;
        (n, w, w) // n items each row, each item with width w, height w
    }
    fn item_center(&self, n: u32, w: u32, h: u32, i: usize) -> (u32, u32) {
        let i: u32 = i as u32;
        let x = THUMB_GAP + w / 2 + (w + THUMB_GAP) * (i % n);
        let y = THUMB_GAP + h / 2 + (h + THUMB_GAP) * (i / n);
        (x, y)
    }
}
impl Display for GalleryView {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        let (n, w, h) = self.get_row_layout();

        canvas.set_clip_rect(rect);
        for (i, img) in self.images.iter().enumerate() {
            let (x, y) = self.item_center(n, w, h, i);
            let r = Rect::from_center(Point::new(x as i32, y as i32), w, h);
            img.borrow().render(canvas, r);
        }
        canvas.set_clip_rect(None);
    }
    fn handle_events(&mut self, evt: &Event) {
        self.gesture_detector.feed(evt);

        // single touch
        for ref event in self.gesture_detector.poll() {
            match event {
                &GestureEvent::Tap(x, y) => {
                    let x = x * (*Config::get_u32("width").unwrap()) as f32;
                    let y = y * (*Config::get_u32("height").unwrap()) as f32;
                    let i = self.image_under_point(x as u32, y as u32);

                    println!("tap! {} {} {:?}", x, y, i);

                    
                },
                &GestureEvent::PanStart { .. } => {
                    self.dragging = true;
                },
                &GestureEvent::Pan { x, y, dx, dy, .. } => {
                },
                &GestureEvent::PanEnd { .. } => {
                    self.dragging = false;
                },
                _ => ()
            }
        }
    }
    fn is_interactive(&self) -> bool {
        true
    }
}

const PREVIEW_GAP: i32 = 30;

pub struct Preview {
    parent: Weak<RefCell<Scene>>,
    prev: Rc<RefCell<ScrollView>>,
    curr: Rc<RefCell<ScrollView>>,
    next: Rc<RefCell<ScrollView>>,
    width: u32,
    height: u32,
    dragging: bool,
    translate_x: i32,
    translate_x_pre: i32,
    img_idx: usize,
    transition: Option<Transition>,
    gesture_detector: GestureDetector,
}

impl Preview {
    pub fn new(parent: Rc<RefCell<Scene>>) -> Rc<RefCell<Preview>> {

        let width = *Config::get_u32("width").unwrap();
        let height = *Config::get_u32("height").unwrap();

        let prev = ScrollView::new(Image::new_with_dimension("".to_owned(), width, height));
        prev.borrow_mut().set_rect(0, 0, width, height);

        let curr = ScrollView::new(Image::new_with_dimension("".to_owned(), width, height));
        curr.borrow_mut().set_rect(0, 0, width, height);

        let next = ScrollView::new(Image::new_with_dimension("".to_owned(), width, height));
        next.borrow_mut().set_rect(0, 0, width, height);

        let mut g = Preview {
            parent: Rc::downgrade(&parent),
            prev,
            curr,
            next,
            width,
            height,
            dragging: false,
            translate_x: 0,
            translate_x_pre: 0,
            img_idx: 0,
            transition: None,
            gesture_detector: GestureDetector::new(),
        };
        g.set_curr_image(0);
        Rc::new(RefCell::new(g))
    }

    fn rotate(&mut self) {
        println!("rotate with translate_x: {}", self.translate_x);
        let p = self.img_idx as isize - 1;
        let config = Config::get_gallery().unwrap();
        if self.translate_x > 0 && p >= 0 {
            println!("rotate left");
            self.translate_x -= self.width as i32 + PREVIEW_GAP;
            self.set_curr_image(p as usize);
        } else if self.translate_x < 0 && self.img_idx + 1 < config.pics.len() {
            println!("rotate right");
            self.translate_x += self.width as i32 + PREVIEW_GAP;
            let i = self.img_idx + 1;
            self.set_curr_image(i);
        }
    }

    fn set_curr_image(&mut self, idx: usize) {
        //  set prev scrollview
        let mut scrollview = self.prev.borrow_mut();
        let config = Config::get_gallery().unwrap();

        {
            let mut img = scrollview.content.borrow_mut();

            let i = idx as isize - 1;
            if i < 0 {
                img.set_src("");
            } else if let Some(pic) = config.pics.get(i as usize) {
                img.set_src(&pic.url);
            } else {
                img.set_src("");
            }
        }
        scrollview.reset();

        //  set curr scrollview
        let mut scrollview = self.curr.borrow_mut();
        {
            let mut img = scrollview.content.borrow_mut();
            if let Some(pic) = config.pics.get(idx) {
                img.set_src(&pic.url);
            } else {
                img.set_src("");
            }
        }
        scrollview.reset();

        //  set next scrollview
        let mut scrollview = self.next.borrow_mut();
        {
            let mut img = scrollview.content.borrow_mut();
            if let Some(pic) = config.pics.get(idx + 1) {
                img.set_src(&pic.url);
            } else {
                img.set_src("");
            }
        }
        scrollview.reset();

        self.img_idx = idx;
    }

    fn move_to(&mut self, x: i32, duration: Duration) {
        self.transition = Some(Transition::new(self.translate_x,
                                               x,
                                               duration));
    }
}

impl Display for Preview {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        if self.translate_x > 0 {
            let mut r0 = rect.clone();
            r0.offset(self.translate_x - self.width as i32 - PREVIEW_GAP, 0);
            self.prev.borrow().render(canvas, r0);
        }

        let mut r1 = rect.clone();
        r1.offset(self.translate_x, 0);
        self.curr.borrow().render(canvas, r1);

        if self.translate_x < 0 {
            let mut r2 = rect.clone();
            r2.offset(self.translate_x + self.width as i32 + PREVIEW_GAP, 0);
            self.next.borrow().render(canvas, r2);
        }
    }
    fn handle_events(&mut self, evt: &Event) {
        let config = Config::get_gallery().unwrap();
        self.gesture_detector.feed(evt);

        // single touch
        for ref event in self.gesture_detector.poll() {
            {
                let mut scrollview = self.curr.borrow_mut();
                match event {
                    // double tap gesture
                    &GestureEvent::DoubleTap(..) => {
                        if scrollview.zoom_mode {
                            // exit zoom
                            scrollview.exit_zoom();
                        } else {
                            scrollview.enter_zoom();
                        }
                    },
                    _ => ()
                }
            }

            // handle horizontal move
            {
                match event {
                    &GestureEvent::PanStart { .. } => {
                        self.dragging = true;
                        self.transition = None;
                        self.translate_x_pre = self.translate_x;
                    },
                    &GestureEvent::Pan { x, y, mut dx, mut dy, .. } => {
                        dx = dx * self.width as f32;
                        dy = dy * self.height as f32;

                        let mut scrollview = self.curr.borrow_mut();
                        // if move is in opposite direction with outer tranlation
                        // let out consume minimal move to to restore outer position to 0
                        if self.translate_x > 0 && dx < 0. || self.translate_x < 0 && dx > 0. {
                            let moved = dx.signum() * dx.abs().min((self.translate_x as f32).abs());
                            // move outer
                            self.translate_x += moved as i32;
                            dx -= moved;
                        }

                        // then inner accept remaining move
                        if scrollview.zoom_mode {
                            // move inner
                            let remain = scrollview.move_by(dx, dy);
                            dx = remain.0;
                        }

                        // outer again accept remaining move
                        self.translate_x += dx as i32;
                    },
                    &GestureEvent::PanEnd { .. } => {
                        self.dragging = false;
                        // move direction: -1 to left, 1 to right, 0 restore
                        let delta = self.translate_x - self.translate_x_pre;
                        let threshold = 50; // threshold for the move
                        let mut mov = if delta > threshold {
                            1
                        } else if delta < -threshold {
                            -1
                        } else {
                            0
                        };

                        // duel with invalid move for the first slide and the last
                        if mov == -1 && self.img_idx >= config.pics.len() - 1
                            || mov == 1 && self.img_idx == 0 {
                                mov = 0;
                            }
                        let target_x = mov * (self.width as i32 + PREVIEW_GAP);
                        self.move_to(target_x, Duration::from_millis(300));
                    },
                    _ => (),
                }
            }
        }

        let fingers = num_touch_fingers(1);
        if fingers >= 2 {
            // multi touch
            let mut scrollview = self.curr.borrow_mut();
            match evt {
                // pinch gesture
                &Event::MultiGesture {x, y, d_dist, ..} => {
                    scrollview.scale_by(x * self.width as f32,
                                        y * self.height as f32,
                                        d_dist * 5.);
                },
                _ => ()
            }
        }
    }
    fn update(&mut self) {
        // update scrollview slide animation
        if !self.dragging {
            let mut scrollview = self.curr.borrow_mut();
            if scrollview.zoom_mode {
                scrollview.update();
            }
        }

        // check Preview horizontal slide end
        let mut in_transition = !self.dragging && self.transition.is_some();
        if in_transition {
            if let Some(ref mut transition) = self.transition {
                self.translate_x = transition.step() as i32;
                if transition.at_end() {
                    // end transition
                    in_transition = false;
                    // self.transition_x = transition.target_val();
                }
            }
            if !in_transition {
                self.transition = None;
                self.rotate();
            }
        }
    }
    fn is_interactive(&self) -> bool {
        true
    }
}

pub struct ScrollView {
    pub content: Rc<RefCell<Image>>,
    rect: Rect,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    offset_x_limit: f32,
    offset_y_limit: f32,
    zoom_mode: bool,
    dx: f32,
    dy: f32,
    // these mean are to track mean move speed
    mean_x: Mean<f32>,
    mean_y: Mean<f32>,
}

impl ScrollView {
    fn new(content: Rc<RefCell<Image>>) -> Rc<RefCell<ScrollView>> {
        Rc::new(RefCell::new(ScrollView {
            content,
            rect: Rect::new(0, 0, 0, 0),
            scale: 1.0,
            offset_x: 0.,
            offset_y: 0.,
            offset_x_limit: 0.,
            offset_y_limit: 0.,
            zoom_mode: false,
            dx: 0.,
            dy: 0.,
            mean_x: Mean::new(3),
            mean_y: Mean::new(3),
        }))
    }

    pub fn reset(&mut self) {
        self.set_scale(1.);
        self.offset_x = 0.;
        self.offset_y = 0.;
    }

    fn enter_zoom(&mut self) {
        self.cover();
        self.zoom_mode = true;
    }

    fn exit_zoom(&mut self) {
        self.contain();
        self.offset_x = 0.;
        self.offset_y = 0.;
        self.zoom_mode = false;
    }

    fn scale_by(&mut self, x: f32, y: f32, d: f32) {
        let r = self.scale;
        self.set_scale((r + d).max(0.1));
    }

    fn update(&mut self) {
        if self.dx.abs() < 0.00001 && self.dy.abs() < 0.00001 {
            self.dx = 0.;
            self.dy = 0.;
            // slide stopped
            return;
        }

        let offset_x = self.offset_x + self.dx;
        let offset_y = self.offset_y + self.dy;
        self.set_pos(offset_x, offset_y);

        let friction = 0.7;
        self.dx = apply_friction(friction, self.dx);
        self.dy = apply_friction(friction, self.dy);
    }

    fn set_rect(&mut self, x: i32, y: i32, w: u32, h: u32) {
        self.rect.set_x(x);
        self.rect.set_y(y);
        self.rect.set_width(w);
        self.rect.set_height(h);
    }

    fn set_scale(&mut self, scale: f32) {
        let w = self.rect.width();
        let h = self.rect.height();

        if let Some((img_w, img_h)) = self.content.borrow().get_img_size() {
            let (w2, h2) = Image::contain_size(img_w, img_h, w, h);
            self.offset_x_limit = (scale * w2 as f32 - w as f32).max(0.) / 2.;
            self.offset_y_limit = (scale * h2 as f32 - h as f32).max(0.) / 2.;
        }

        self.scale = scale;
        self.zoom_mode = scale > 1.0;
    }

    fn contain(&mut self) {
        // scale 1.0 is contain size
        self.set_scale(1.);
    }

    fn cover(&mut self) {
        let w = self.rect.width();
        let h = self.rect.height();
        let mut r = 2.;
        if let Some((img_w, img_h)) = self.content.borrow().get_img_size() {
            let (w1, _) = Image::cover_size(img_w, img_h, w, h);
            let (w2, _) = Image::contain_size(img_w, img_h, w, h);
            r = w1 as f32 / w2 as f32;
        }

        self.set_scale(r);
        self.dx = 0.;
        self.dy = 0.;
    }

    fn set_pos(&mut self, mut x: f32, mut y: f32) {
        let mut x_limited = true;
        let mut y_limited = true;
        let offset_x_limit = self.offset_x_limit;
        let offset_y_limit = self.offset_y_limit;

        if x > offset_x_limit {
            x = offset_x_limit;
        } else if x < -offset_x_limit {
            x = -offset_x_limit;
        } else {
            x_limited = false;
        }
        self.offset_x = x;

        if y > offset_y_limit {
            y = offset_y_limit;
        } else if y < -offset_y_limit {
            y = -offset_y_limit;
        } else {
            y_limited = false;
        }
        self.offset_y = y;
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) -> (f32, f32) {
        let offset_x = self.offset_x + dx;
        let offset_y = self.offset_y + dy;
        self.set_pos(offset_x, offset_y);

        // get a mean to calc motion speed
        self.mean_x.push(dx);
        self.mean_y.push(dy);

        if self.offset_x_limit == self.offset_x.abs() {
            self.dx = 0.;
        } else {
            self.dx = self.mean_x.get() as f32;
        }

        if self.offset_y_limit == self.offset_y.abs() {
            self.dy = 0.;
        } else {
            self.dy = self.mean_y.get() as f32;
        }
        (offset_x - self.offset_x, offset_y - self.offset_y)
    }
}

impl Display for ScrollView {
    fn render(&self, canvas: &mut Canvas<Window>, rect: Rect) {
        canvas.set_clip_rect(rect);
        let r = Rect::from_center(rect.center().offset(self.offset_x as i32, self.offset_y as i32),
                                  (rect.width() as f32 * self.scale) as u32,
                                  (rect.height() as f32 * self.scale) as u32);
        self.content.borrow().render(canvas, r);
        canvas.set_clip_rect(None);
    }
}

fn apply_friction(friction: f32, dx: f32) -> f32 {
    if dx.abs() < friction {
        0f32
    } else {
        dx + (if dx > 0f32 {-friction} else {friction})
    }
}
