use ash::vk;
use nalgebra_glm::TMat4;

use crate::buffer::Buffer;

pub struct Model {
    pub name: String,
    pub transform: nalgebra_glm::TMat4<f32>,
    pub mesh: Mesh,
}

impl Model {
    pub fn new(
        name: String,
        transform: nalgebra_glm::TMat4<f32>,
        primitives: Vec<Primitive>,
    ) -> Self {
        let mesh = Mesh { primitives };
        Self {
            name,
            transform,
            mesh,
        }
    }
}

pub struct Mesh {
    pub primitives: Vec<Primitive>,
}

pub struct Primitive {
    pub offset: u32,
    pub num_indices: u32,
}

pub fn import_models(
    device: &ash::Device,
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> (Buffer<Vertex>, Buffer<u32>, Vec<Model>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let (gltf, buffers, _images) = gltf::import("assets/BoomBoxWithAxes.gltf").unwrap();
    let mut models = Vec::new();
    let mut index_offset = 0;

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            import_node(
                node,
                &mut indices,
                &mut vertices,
                &buffers,
                &mut models,
                &mut index_offset,
                &nalgebra_glm::identity(),
            );
        }
    }

    let vertex_buffer = unsafe {
        Buffer::new(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            &vertices,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )
    };

    let index_buffer = unsafe {
        Buffer::new(
            device,
            instance,
            physical_device,
            descriptor_pool,
            descriptor_set_layout,
            &indices,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )
    };

    (vertex_buffer, index_buffer, models)
}

fn import_node(
    node: gltf::Node,
    indices: &mut Vec<u32>,
    vertices: &mut Vec<Vertex>,
    blob: &[gltf::buffer::Data],
    models: &mut Vec<Model>,
    offset: &mut u32,
    parent_transform: &nalgebra_glm::TMat4<f32>,
) {
    let local_transform: TMat4<f32> = node.transform().matrix().into();
    if let Some(mesh) = node.mesh() {
        let mut primitives = Vec::new();
        let transform = parent_transform * &local_transform;

        let name = if let Some(name) = node.name() {
            name.to_string()
        } else {
            format!("Node {}", node.index())
        };

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|b| Some(&blob[b.index()]));
            for i in reader.read_indices().unwrap().into_u32() {
                indices.push(i);
            }

            for position in reader.read_positions().unwrap() {
                vertices.push(Vertex::new(position[0], position[1], position[2]));
            }

            if let Some(colours) = reader.read_colors(0) {
                for (colour, position) in
                    colours.into_rgb_f32().zip(reader.read_positions().unwrap())
                {
                    vertices.push(Vertex::new_coloured(
                        position[0],
                        position[1],
                        position[2],
                        colour[0],
                        colour[1],
                        colour[2],
                    ));
                }
            } else {
                for position in reader.read_positions().unwrap() {
                    vertices.push(Vertex::new(position[0], position[1], position[2]));
                }
            }

            let num_indices = indices.len() as _;
            primitives.push(Primitive {
                offset: *offset as _,
                num_indices,
            });

            *offset += num_indices;
        }

        models.push(Model::new(name, transform, primitives));
    }

    for node in node.children() {
        import_node(
            node,
            indices,
            vertices,
            blob,
            models,
            offset,
            &local_transform,
        );
    }
}

pub struct VertexInputDescription {
    pub bindings: Vec<vk::VertexInputBindingDescription>,
    pub attributes: Vec<vk::VertexInputAttributeDescription>,
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Vertex {
    vx: f32,
    vy: f32,
    vz: f32,
    r: f32,
    g: f32,
    b: f32,
}

impl Vertex {
    pub fn description() -> VertexInputDescription {
        VertexInputDescription {
            bindings: vec![vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<Vertex>() as _,
                input_rate: vk::VertexInputRate::VERTEX,
            }],
            attributes: vec![
                vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: 0,
                },
                vk::VertexInputAttributeDescription {
                    location: 1,
                    binding: 0,
                    format: vk::Format::R32G32B32_SFLOAT,
                    offset: (std::mem::size_of::<f32>() * 3) as u32,
                },
            ],
        }
    }

    pub fn new(vx: f32, vy: f32, vz: f32) -> Self {
        Self {
            vx,
            vy,
            vz,
            ..Default::default()
        }
    }

    pub fn new_coloured(vx: f32, vy: f32, vz: f32, r: f32, g: f32, b: f32) -> Self {
        Self {
            vx,
            vy,
            vz,
            r,
            g,
            b,
            ..Default::default()
        }
    }
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            vx: 0.,
            vy: 0.,
            vz: 0.,
            r: 1.,
            g: 1.,
            b: 1.,
        }
    }
}
