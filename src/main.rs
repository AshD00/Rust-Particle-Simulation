use std::sync::{atomic::AtomicU32, Arc};
//use std::thread::{__FastLocalKeyInner, __OsLocalKeyInner};
use std::time::SystemTime;

use glium::glutin::event::{VirtualKeyCode};
//use glium::glutin::window::Window;
//use glium::{uniforms::UniformsStorage, program};

#[macro_use]
extern crate glium;

const INITIAL_SPEED: f32 = -0.02;
const COOL_FACTOR: f32 = 0.0000015;
const GRAVITY: f32 = 0.0005;
const NUM_OF_THREADS: usize = 12;
const PARTICLES_PER_THREAD: usize = 25;
const COLLISION_DISTANCE: f32 = 0.01;

const PERFORMANCE: bool = false;

#[derive(Debug, Copy, Clone)]

struct Particle {
    x: f32, //x coordinate
    y: f32, //y coordinate
    v: f32, //velocity
    d: f32, //direction
    d_abs: f32, //The absolute value of direction
    m: f32, //size
    t: f32, //temperature
}
impl Particle {
    pub fn new(x_value: f32, y_value: f32, v_value: f32, d_value: f32, m_value: f32, t_value: f32) -> Particle {
        Particle {
            x: x_value,
            y: y_value,
            v: v_value,
            d: d_value,
            d_abs: d_value.abs(),
            m: m_value,
            t: t_value,
        }
    }

    pub fn thread_main(list: &mut [Particle]) -> u32 {
        let mut counter = 0;
        for particle in list {
            if particle.x + particle.d < 0.5 && particle.x + particle.d > -0.5 {
                particle.x += particle.d;    
            }
            else {
                particle.d -= particle.d*1.8;
            }

            particle.v -= GRAVITY;

            if particle.y + particle.v - particle.d_abs/10.0 > -1.0 {
                particle.y += particle.v - particle.d_abs/10.0;    
            }    
            else {
                let mut x:f32 = rand::random();
                x -= 0.5;

                if particle.y > -1.9 {
                    counter += 1;
                }

                particle.d = x*0.025;
                particle.d_abs = particle.d.abs();

                particle.y = 1.0;
                particle.v = INITIAL_SPEED;
                particle.x = x*0.3;
                particle.m = 1.0;
                particle.t = 1.0;
            }
        }
        return counter
    }

    pub fn temp_thread(list: &mut [Particle], delta_t: f32) {
        for particle in list {
            let new_temp = particle.t - delta_t * COOL_FACTOR / particle.m;
            if new_temp >= 0.0 && new_temp <= 1.0 {
                particle.t = new_temp
            }
            else if new_temp < 0.0 {
                particle.t = 0.0;
            }
            else if new_temp > 1.0 {
                particle.t = 1.0;
            }
        }
    }

    pub fn collision_thread(list: &mut [Particle]) -> u32 {
        let mut collisions = 0;
        let length = list.len();
        let mut i = 0;
        let mut x = 1;
        
        while i < length {
            while x < length {
                if x != i {
                    let particle2 = list[x];
                    if list[i].collide(&particle2) {
                        collisions += 1;

                        list[i].v = (list[i].m * list[i].v + list[x].m * list[x].v) / (list[i].m + list[x].m);
                        list[i].m += list[x].m;
                        list[x].y = -2.0;
                    }
                }
                x += 1;
            }

            i += 1;
            x = i;
        }
        //println!("Number of collisions in thread: {}", collisions);
        return collisions;
    }

    pub fn collide(&mut self, test_particle: &Particle) -> bool {
        if self.x -COLLISION_DISTANCE <= test_particle.x && test_particle.x <= self.x + COLLISION_DISTANCE && self.y -COLLISION_DISTANCE <= test_particle.y && test_particle.y <= self.y + COLLISION_DISTANCE {
            return true;
        }
        return false;
    }
}
 
struct ParticleSystem {
    particles: Vec<Particle>,
}
impl ParticleSystem {
    pub fn new() -> ParticleSystem {
        ParticleSystem {
            particles: Vec::new(),
        }
    }

    //Fixed
    pub fn move_particle(&mut self, delta_t: f32) -> u32 {
        let mut pool = scoped_threadpool::Pool::new(NUM_OF_THREADS as u32);
        let counter = Arc::new(AtomicU32::new(0));

        pool.scoped(|scope| {
            for slice in self.particles.chunks_mut(PARTICLES_PER_THREAD) {
                let counter_clone = counter.clone();
                scope.execute(move || {
                    counter_clone.fetch_add(Particle::thread_main(slice), std::sync::atomic::Ordering::SeqCst);
                    Particle::collision_thread(slice);
                    Particle::temp_thread(slice, delta_t);
                });
            }
        });
        return counter.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn move_particle_basic(&mut self) -> u32 {
        let mut pool = scoped_threadpool::Pool::new(NUM_OF_THREADS as u32);
        let counter = Arc::new(AtomicU32::new(0));

        pool.scoped(|scope| {
            for slice in self.particles.chunks_mut(PARTICLES_PER_THREAD) {
                let counter_clone = counter.clone();
                scope.execute(move || {
                    counter_clone.fetch_add(Particle::thread_main(slice), std::sync::atomic::Ordering::SeqCst);
                });
            }
        });
        return counter.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn temp_particle(&mut self, delta_t: f32) {
        let mut pool = scoped_threadpool::Pool::new(NUM_OF_THREADS as u32);

        pool.scoped(|scope| {
            for slice in self.particles.chunks_mut(PARTICLES_PER_THREAD) {
                scope.execute(move || {
                    Particle::temp_thread(slice, delta_t);
                });
            }
        });
    }

    pub fn collide_particle(&mut self) {
        let mut pool = scoped_threadpool::Pool::new(NUM_OF_THREADS as u32);

        pool.scoped(|scope| {
            for slice in self.particles.chunks_mut(PARTICLES_PER_THREAD) {
                scope.execute(move || {
                    Particle::collision_thread(slice);
                });
            }
        });
    }
}

fn main() {
    #[allow(unused_imports)]
    use glium::{glutin, Surface};

    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
    }

    implement_vertex!(Vertex, position);

    let vertex1 = Vertex { position: [-0.05, -0.0288] };
    let vertex2 = Vertex { position: [ 0.00,  0.0577] };
    let vertex3 = Vertex { position: [ 0.05, -0.0288] };
    let shape = vec![vertex1, vertex2, vertex3];

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 140

        in vec2 position;

        uniform mat4 matrix;

        uniform vec4 colours;
        out vec4 c;

        void main() {
            gl_Position = matrix * vec4(position, 0.0, 1.0);
            c = colours;
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec4 c;
        out vec4 color;

        void main() {
            color = c;
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let mut system1 = ParticleSystem::new();
    
    let mut floor_collisions = 0;
    
    let mut temp = true;

    let mut x = -0.4;
    let mut y = -0.4;
    let mut z = 0;
    let mut direction;
    
    let mut num_iter = 0;
    
    if PERFORMANCE {
        num_iter = 4999;
    }

    let mut start = SystemTime::now();

    while z <= num_iter {
        while x < 0.5 {
            while y < 0.5 {
                direction = (y*0.006)+(x*0.02);
                let p = Particle::new(x*0.2, 1.0, INITIAL_SPEED, direction, 1.0, 1.0);
                system1.particles.push(p);
                y+=0.1;
            }
            y = -0.4;
            x += 0.1;
        }
        x = -0.4;
        y = -0.4;
        z += 1;    
    }


    //println!("Number of particles: {}", system1.particles.len());

    event_loop.run(move |event, _, control_flow| {

        match event {
            glutin::event::Event::WindowEvent { event, .. } => match event {
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                    return;
                },
                glutin::event::WindowEvent::KeyboardInput { input, .. } => {
                    //Press T to display temperature
                    if input.virtual_keycode == Some(VirtualKeyCode::T) {
                        temp = true;
                    }
                    //Press M to display Mass
                    else if input.virtual_keycode == Some(VirtualKeyCode::M) {
                        temp = false;
                    }
                    //Press F to print the number of particles that have hit the floor
                    else if input.virtual_keycode == Some(VirtualKeyCode::F) {
                        println!("Number of collisions with the floor: {}", floor_collisions);
                    }
                    //Press N to print the number of particles
                    else if input.virtual_keycode == Some(VirtualKeyCode::N) {
                        println!("Number of particles: {}", system1.particles.len());
                    }
                    return;
                },
                _ => return,
            },
            glutin::event::Event::NewEvents(cause) => match cause {
                glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = glutin::event_loop::ControlFlow::WaitUntil(next_frame_time);

        // Begin render loop

        // Create a drawing target
        let mut target = display.draw();

        // Clear the screen to black
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        
        let duration = start.elapsed().unwrap().as_micros();
        if PERFORMANCE {
            floor_collisions += system1.move_particle(duration as f32);
        }
        else {
            floor_collisions += system1.move_particle_basic();
            system1.collide_particle();
    
            system1.temp_particle(duration as f32);    
        }
        start = SystemTime::now();

        //println!("List size: {}", system1.particles.len());


        let mut i = 0;
        for p in &system1.particles {
            i += 1;
            if i != 5000 && PERFORMANCE {
                continue;
            }
            else if temp {
                //Coloured correlating to temperature, the closer to blue it is, the colder it is (i.e. the longer its fallen)
                let uniforms = uniform! {
                    matrix: [
                        [0.3, 0.0, 0.0, 0.0],
                        [0.0, 0.3, 0.0, 0.0],
                        [0.0, 0.0, 0.5, 0.0],
                        [p.x, p.y, 1.0, 1.0],
                    ],
                    colours: [p.t, 0.0, 1.0-p.t, 1.0]
                };    
                target.draw(&vertex_buffer, &indices, &program, &uniforms, &Default::default()).unwrap();
                i = 0;
            }
            else {
                //Coloured correlating to mass, the closer to green it is, the more collisions it's had
                let uniforms = uniform! {
                    matrix: [
                        [0.3, 0.0, 0.0, 0.0],
                        [0.0, 0.3, 0.0, 0.0],
                        [0.0, 0.0, 0.5, 0.0],
                        [p.x, p.y, 1.0, 1.0],
                    ],
                    colours: [0.0, -0.3+p.m/2.0, 1.0-p.m/2.0, 1.0]
                };    
                target.draw(&vertex_buffer, &indices, &program, &uniforms, &Default::default()).unwrap();
                i = 0;
            }
        }
        //println!("Number of particles on screen: {}", i-n);
        
        // Display the completed drawing
        target.finish().unwrap();

        // End render loop
    });
}
