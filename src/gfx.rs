//! A minimal renderer on miniquad: solid rectangles and circles, batched.
//!
//! The game draws nothing but flat shapes, so one untextured shader and one
//! buffer cover it. Doing this here rather than through macroquad skips the
//! font atlases, UI and extra shaders macroquad sets up at start.

use miniquad::{
    Bindings, BlendFactor, BlendState, BlendValue, BufferLayout, BufferSource, BufferType,
    BufferUsage, Equation, PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderMeta,
    ShaderSource, UniformBlockLayout, UniformDesc, UniformType, UniformsSource, VertexAttribute,
    VertexFormat,
};

use crate::view;

/// Vertices per batch: 1000 rectangles, more than a busy frame draws, so a
/// frame is nearly always a single draw call.
const MAX_VERTICES: usize = 4000;
const MAX_INDICES: usize = MAX_VERTICES / 4 * 6;
/// Triangles around a circle; plenty for the 16 px moon.
const CIRCLE_SEGMENTS: usize = 24;

const VERTEX: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec4 in_color;
uniform vec4 transform;
varying lowp vec4 color;
void main() {
    gl_Position = vec4(in_pos * transform.xy + transform.zw, 0.0, 1.0);
    color = in_color;
}"#;

const FRAGMENT: &str = r#"#version 100
varying lowp vec4 color;
void main() {
    gl_FragColor = color;
}"#;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    color: Color,
}

pub struct Gfx {
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    /// Scale and offset from playfield units to clip space.
    transform: [f32; 4],
}

impl Gfx {
    pub fn new() -> Self {
        let mut ctx = miniquad::window::new_rendering_backend();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("transform", UniformType::Float4)],
                    },
                },
            )
            .expect("the built-in shader compiles");
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_color", VertexFormat::Float4),
            ],
            shader,
            PipelineParams {
                // The banner, the stars and the moon fade in and out.
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Vertex>(MAX_VERTICES),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<u16>(MAX_INDICES),
        );
        Self {
            ctx,
            pipeline,
            bindings: Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![],
            },
            vertices: Vec::with_capacity(MAX_VERTICES),
            indices: Vec::with_capacity(MAX_INDICES),
            transform: [1.0, 1.0, 0.0, 0.0],
        }
    }

    /// Clears the window and maps `area` of the playfield onto all of it.
    pub fn begin(&mut self, clear: Color, area: view::Rect) {
        self.ctx
            .begin_default_pass(PassAction::clear_color(clear.r, clear.g, clear.b, 1.0));
        self.transform = transform(area);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let base = self.reserve(4, 6);
        for (vx, vy) in [(x, y), (x + w, y), (x + w, y + h), (x, y + h)] {
            self.vertices.push(Vertex {
                pos: [vx, vy],
                color,
            });
        }
        self.indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32, color: Color) {
        let base = self.reserve(CIRCLE_SEGMENTS + 1, CIRCLE_SEGMENTS * 3);
        self.vertices.push(Vertex { pos: [x, y], color });
        for i in 0..CIRCLE_SEGMENTS {
            let angle = i as f32 / CIRCLE_SEGMENTS as f32 * std::f32::consts::TAU;
            self.vertices.push(Vertex {
                pos: [x + r * angle.cos(), y + r * angle.sin()],
                color,
            });
            let next = (i + 1) % CIRCLE_SEGMENTS;
            self.indices
                .extend([base, base + 1 + i as u16, base + 1 + next as u16]);
        }
    }

    /// Draws what is batched and shows the frame.
    pub fn end(&mut self) {
        self.flush();
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    /// Makes room for a shape, drawing the batch first if it is full, and
    /// returns the index its first vertex will have.
    fn reserve(&mut self, vertices: usize, indices: usize) -> u16 {
        if self.vertices.len() + vertices > MAX_VERTICES
            || self.indices.len() + indices > MAX_INDICES
        {
            self.flush();
        }
        self.vertices.len() as u16
    }

    fn flush(&mut self) {
        if self.indices.is_empty() {
            return;
        }
        let vertex_buffer = self.bindings.vertex_buffers[0];
        self.ctx
            .buffer_update(vertex_buffer, BufferSource::slice(&self.vertices));
        self.ctx.buffer_update(
            self.bindings.index_buffer,
            BufferSource::slice(&self.indices),
        );
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx
            .apply_uniforms(UniformsSource::table(&self.transform));
        self.ctx.draw(0, self.indices.len() as i32, 1);
        self.vertices.clear();
        self.indices.clear();
    }
}

/// Scale and offset that take `area` to clip space, with y pointing down as
/// it does in the playfield.
fn transform(area: view::Rect) -> [f32; 4] {
    let sx = 2.0 / area.w;
    let sy = -2.0 / area.h;
    [sx, sy, -1.0 - area.x * sx, 1.0 - area.y * sy]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_visible_area_fills_clip_space_top_down() {
        let t = transform(view::Rect {
            x: -100.0,
            y: 20.0,
            w: 1000.0,
            h: 400.0,
        });
        let near = |x: f32, y: f32, want: (f32, f32)| {
            let got = (x * t[0] + t[2], y * t[1] + t[3]);
            assert!(
                (got.0 - want.0).abs() < 1e-5 && (got.1 - want.1).abs() < 1e-5,
                "{got:?}"
            );
        };
        near(-100.0, 20.0, (-1.0, 1.0));
        near(900.0, 420.0, (1.0, -1.0));
    }
}
