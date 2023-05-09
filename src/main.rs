extern crate ncollide3d as ncollide;
extern crate rayon;
extern crate sdl2;
extern crate wavefront_obj as obj;

use rayon::prelude::*;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::{PixelFormatEnum};

use ncollide::na::{Isometry3, Point3, Translation3, UnitQuaternion, Vector3};
use ncollide::pipeline::FirstInterferenceWithRay;

use ncollide::query::Ray;
use ncollide::shape::{Ball, ShapeHandle, TriMesh, Triangle};
use ncollide::world::{CollisionWorld};
use ncollide::pipeline::object::{GeometricQueryType, CollisionGroups};

use obj::obj::Primitive;

struct WorldObjectData {
    color: u8,
}

impl WorldObjectData {
    fn new(color: u8) -> Self {
        Self { color }
    }
}

fn main() {
    println!("Using {} threads.\n", rayon::current_num_threads());

    let video_scale = 4;
    let (video_width, video_height) = (1280 / video_scale, 720 / video_scale);

    // Init SDL
    let ctx = sdl2::init().unwrap();
    ctx.mouse().set_relative_mouse_mode(true);
    let video = ctx.video().unwrap();

    let window = video
        .window(
            "The City",
            video_width * video_scale,
            video_height * video_scale,
        )
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut screen = texture_creator
        .create_texture_streaming(Some(PixelFormatEnum::RGB332), video_width, video_height)
        .unwrap();
    let mut pixels = vec![0u8; video_width as usize * video_height as usize];
    let mut event_pump = ctx.event_pump().unwrap();

    // Init world
    let groups = CollisionGroups::new();
    let query = GeometricQueryType::Contacts(0.001, 0.09);
    let mut world = CollisionWorld::<f32, WorldObjectData>::new(0.02);
    let ball_shape = ShapeHandle::new(Ball::new(0.5));
    let triangle_shape = ShapeHandle::new(Triangle::new(
        Point3::new(-0.5, -0.5, 0.0),
        Point3::new(0.0, 0.5, 0.0),
        Point3::new(0.5, -0.5, 0.0),
    ));

    world.add(
        Isometry3::from_parts(Translation3::new(1.0, 2.0, 2.0), UnitQuaternion::identity()),
        ball_shape.clone(),
        groups,
        query,
        WorldObjectData::new(0x11),
    );
    world.add(
        Isometry3::from_parts(
            Translation3::new(-1.0, 2.0, 2.0),
            UnitQuaternion::identity(),
        ),
        ball_shape.clone(),
        groups,
        query,
        WorldObjectData::new(0x22),
    );

    let (obj3, ..) = world.add(
        Isometry3::from_parts(Translation3::new(0.0, 4.0, 2.0), UnitQuaternion::identity()),
        triangle_shape.clone(),
        groups,
        query,
        WorldObjectData::new(0x33),
    );

    if let Some(contents) = read_file("The City/The City.obj") {
        match obj::obj::parse(contents) {
            Ok(obj_set) => {
                for obj in obj_set.objects.iter() {
                    let vertices: Vec<_> = obj
                        .vertices
                        .iter()
                        .map(|v| Point3::new(v.x as f32, v.y as f32, v.z as f32))
                        .collect();
                    let indices: Vec<_> = obj
                        .geometry
                        .iter()
                        .flat_map(|g| {
                            g.shapes
                                .iter()
                                .map(|s| {
                                    if let Primitive::Triangle(v1, v2, v3) = s.primitive {
                                        Some(Point3::new(v1.0, v2.0, v3.0))
                                    } else {
                                        None
                                    }
                                })
                                .filter(Option::is_some)
                                .map(Option::unwrap)
                        })
                        .collect();

                    let shape = TriMesh::new(vertices, indices, None);
                    let iso = Isometry3::from_parts(
                        Translation3::new(0.0, -1.0, 10.0),
                        UnitQuaternion::identity(),
                    );
                    world.add(
                        iso,
                        ShapeHandle::new(shape),
                        groups,
                        query,
                        WorldObjectData::new(0xDD),
                    );
                }
            }
            Err(e) => println!("Failed to parse map: '{:?}'", e),
        }
    } else {
        println!("Failed to open map.");
    }

    world.update();

    let mut camera_pos = Point3::new(0.0, 0.0, 0.0);
    let mut camera_rot = Vector3::new(0.0, 0.0, 0.0);
    let mut light_pos = Point3::new(0.0, 10.0, 2.0);

    let start_time = std::time::Instant::now();

    'main: loop {
        let elapsed = start_time.elapsed();
        let elapsed = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9;
        let elapsed = elapsed as f32;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }

        let speed = 0.1;
        let ks = event_pump.keyboard_state();
        let rms = event_pump.relative_mouse_state();

        // Update camera rotation
        camera_rot += Vector3::new(
            rms.y() as f32 / video_height as f32,
            rms.x() as f32 / video_width as f32,
            0.0,
        );

        // Update camera position
        if ks.is_scancode_pressed(Scancode::W) {
            let rot = camera_rot.y;
            camera_pos += Vector3::new(rot.sin(), 0.0, rot.cos()) * speed;
        }
        if ks.is_scancode_pressed(Scancode::S) {
            let rot = camera_rot.y + 180f32.to_radians();
            camera_pos += Vector3::new(rot.sin(), 0.0, rot.cos()) * speed;
        }
        if ks.is_scancode_pressed(Scancode::A) {
            let rot = camera_rot.y - 90f32.to_radians();
            camera_pos += Vector3::new(rot.sin(), 0.0, rot.cos()) * speed;
        }
        if ks.is_scancode_pressed(Scancode::D) {
            let rot = camera_rot.y + 90f32.to_radians();
            camera_pos += Vector3::new(rot.sin(), 0.0, rot.cos()) * speed;
        }
        if ks.is_scancode_pressed(Scancode::Space) {
            camera_pos += Vector3::new(0.0, 1.0, 0.0) * speed;
        }
        if ks.is_scancode_pressed(Scancode::LCtrl) {
            camera_pos += Vector3::new(0.0, -1.0, 0.0) * speed;
        }

        // Update world objects
        light_pos.x = elapsed.cos() * 4.0;
        light_pos.z = elapsed.sin() * 4.0;

        if let Some(obj3) = world.get_mut(obj3) {
            obj3.set_position(Isometry3::from_parts(
                Translation3::new(0.0, 0.0, 2.0),
                UnitQuaternion::from_euler_angles(elapsed, elapsed / 2.0, 0.0),
            ));
        }

        world.update();

        // Draw frame
        pixels.par_iter_mut().enumerate().for_each(|(i, pix)| {
            let (x, y) = (i as u32 % video_width, i as u32 / video_height);

            // Calculate ray
            let dir = {
                // min: half_missing
                // max: video_width - half_missing
                // map 0..video_width -> half_missing..(video_width - half_missing)
                let (dirx, diry) = {
                    let dirx = x as f32 / video_width as f32;
                    let half_missing = (video_width - video_height) as f32 / 2.0;
                    let diry = (y as f32 + half_missing) / video_width as f32;
                    (dirx, diry)
                };
                let (dirx, diry) = (dirx * 2.0 - 1.0, diry * 2.0 - 1.0);
                Vector3::new(dirx, -diry, 1.0).normalize()
            };

            let dir =
                UnitQuaternion::from_euler_angles(camera_rot.x, camera_rot.y, camera_rot.z) * dir;
            let ray = Ray::new(camera_pos, dir);

            // Find closes object
            let inter = world.first_interference_with_ray(&ray, 100.0, &groups);

            *pix = if let Some(FirstInterferenceWithRay { co: obj, inter: intersec, .. }) = inter {
                // Calculate illumination
                let hit_pos = camera_pos + dir * intersec.toi;
                let dir_to_light = (light_pos - hit_pos).normalize();
                let brightness = intersec.normal.dot(&dir_to_light).max(0.0);

                // Convert color to floats
                let color = obj.data().color;
                let (r, g, b) = (
                    (color & 0b11100000) >> 5,
                    (color & 0b00011100) >> 2,
                    color & 0b00000011,
                );
                let (r, g, b) = (r as f32 / 8.0, g as f32 / 8.0, b as f32 / 3.0);

                // Calculate whether in shadow
                let shadow_coef = {
                    let light_ray = Ray::new(hit_pos + dir_to_light * 0.001, dir_to_light);
                    match world
                        .first_interference_with_ray(&light_ray, 100.0, &groups)
                        .is_some() {
                        true => 0f32,
                        false => 1f32,
                    }
                };

                // Apply illumination / shadow
                let (r, g, b) = (
                    (r * brightness * shadow_coef).min(1.0),
                    (g * brightness * shadow_coef).min(1.0),
                    (b * brightness * shadow_coef).min(1.0),
                );

                // Gamma correction
                let (r, g, b) = (r.powf(1.0 / 2.2), g.powf(1.0 / 2.2), b.powf(1.0 / 2.2));

                // Convert color to u8
                let (r, g, b) = ((r * 8.0) as u8, (g * 8.0) as u8, (b * 3.0) as u8);
                let color = ((r & 0b00000111) << 5) | ((g & 0b00000111) << 2) | (b & 0b00000011);
                color
            } else {
                // Trippy background
                let red = ((x as f32 + elapsed * 20.0 + camera_rot.y * video_width as f32)
                    / video_width as f32
                    * 8.0) as u8;
                let green = ((y as f32 + elapsed * 14.0 + camera_rot.x * video_height as f32)
                    / video_height as f32
                    * 8.0) as u8;
                let pix = ((red & 0b00000111) << 5) | ((green & 0b00000111) << 2);
                pix
            };
        });

        screen.update(None, &pixels, video_width as usize).unwrap();
        canvas.copy(&screen, None, None).unwrap();
        canvas.present();
    }
}

fn read_file<P: AsRef<std::path::Path>>(path: P) -> Option<String> {
    use std::io::Read;

    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return None,
    };

    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);
    Some(contents)
}
