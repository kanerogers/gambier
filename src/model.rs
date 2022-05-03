use std::collections::HashMap;

use ash::vk;
use id_arena::{Arena, Id};
use nalgebra_glm::TMat4;

use crate::{texture::Texture, vulkan_context::VulkanContext};

#[derive(Debug)]
pub struct Model {
    pub name: String,
    pub transform: nalgebra_glm::TMat4<f32>,
    pub mesh: Id<Mesh>,
}

impl Model {
    pub fn new(name: String, transform: nalgebra_glm::TMat4<f32>, mesh: Id<Mesh>) -> Self {
        Self {
            name,
            transform,
            mesh,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub primitives: Vec<Primitive>,
}

#[derive(Debug)]
pub struct Primitive {
    pub index_offset: u32,
    pub vertex_offset: u32,
    pub num_indices: u32,
    pub material: Id<Material>,
}

#[derive(Debug)]
pub struct Material {}

pub struct ImportState<'a> {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    index_offset: u32,
    vertex_offset: u32,
    buffers: Vec<gltf::buffer::Data>,
    images: Vec<gltf::image::Data>,
    models: Vec<Model>,
    vulkan_context: &'a VulkanContext,
    meshes: Arena<Mesh>,
    mesh_ids: HashMap<usize, Id<Mesh>>,
    materials: Arena<Material>,
    material_ids: HashMap<usize, Id<Material>>,
}

impl<'a> ImportState<'a> {
    pub fn new(
        buffers: Vec<gltf::buffer::Data>,
        images: Vec<gltf::image::Data>,
        vulkan_context: &'a VulkanContext,
    ) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            models: Vec::new(),
            index_offset: 0,
            vertex_offset: 0,
            buffers,
            images,
            vulkan_context,
            meshes: Arena::new(),
            mesh_ids: HashMap::new(),
            materials: Arena::new(),
            material_ids: HashMap::new(),
        }
    }
}

pub fn import_models(vulkan_context: &VulkanContext) -> (Vec<Model>, Arena<Mesh>) {
    let (gltf, buffers, images) = gltf::import("assets/BoomBoxWithAxes.gltf").unwrap();
    let mut import_state = ImportState::new(buffers, images, vulkan_context);

    for material in gltf.materials() {
        if let Some(index) = material.index() {
            let material = import_material(material, &mut import_state);
            let id = import_state.materials.alloc(material);
            import_state.material_ids.insert(index, id);
        }
    }

    for mesh in gltf.meshes() {
        let mut primitives = Vec::new();
        for primitive in mesh.primitives() {
            import_primitive(primitive, &mut primitives, &mut import_state);
        }

        let id = import_state.meshes.alloc(Mesh { primitives });
        import_state.mesh_ids.insert(mesh.index(), id);
    }

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            import_node(node, &mut import_state, &nalgebra_glm::identity());
        }
    }

    // Copy indices and vertices into buffers.
    unsafe {
        vulkan_context.index_buffer.overwrite(&import_state.indices);
        vulkan_context
            .vertex_buffer
            .overwrite(&import_state.vertices);
    };

    (import_state.models, import_state.meshes)
}

fn import_node(
    node: gltf::Node,
    import_state: &mut ImportState,
    parent_transform: &nalgebra_glm::TMat4<f32>,
) {
    let local_transform: TMat4<f32> = node.transform().matrix().into();
    let transform = parent_transform * &local_transform;
    let name = if let Some(name) = node.name() {
        name.to_string()
    } else {
        format!("Node {}", node.index())
    };

    if let Some(mesh) = node.mesh() {
        let mesh = import_state.mesh_ids.get(&mesh.index()).unwrap().clone();
        import_state.models.push(Model::new(name, transform, mesh));
    }

    for node in node.children() {
        import_node(node, import_state, &local_transform);
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    primitives: &mut Vec<Primitive>,
    import_state: &mut ImportState,
) {
    if let Some(material_index) = primitive.material().index() {
        let (num_indices, num_vertices) = import_geometry(&primitive, import_state);
        let material = import_state
            .material_ids
            .get(&material_index)
            .unwrap()
            .clone();

        primitives.push(Primitive {
            index_offset: import_state.index_offset,
            vertex_offset: import_state.vertex_offset,
            num_indices,
            material,
        });

        import_state.index_offset += num_indices;
        import_state.vertex_offset += num_vertices;
    }
}

fn import_material(material: gltf::Material, import_state: &mut ImportState) -> Material {
    if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
        let image = &import_state.images[texture.texture().source().index()];
        let _texture = unsafe { Texture::new(import_state.vulkan_context, image) };
    }

    Material {}
}

fn import_geometry(primitive: &gltf::Primitive, import_state: &mut ImportState) -> (u32, u32) {
    let buffers = &import_state.buffers;
    let reader = primitive.reader(|b| Some(&buffers[b.index()]));
    let mut num_indices = 0;
    let mut num_vertices = 0;
    for i in reader.read_indices().unwrap().into_u32() {
        num_indices += 1;
        import_state.indices.push(i);
    }
    if let Some(colours) = reader.read_colors(0) {
        for (colour, position) in colours.into_rgb_f32().zip(reader.read_positions().unwrap()) {
            num_vertices += 1;
            import_state.vertices.push(Vertex::new_coloured(
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
            num_vertices += 1;
            import_state
                .vertices
                .push(Vertex::new(position[0], position[1], position[2]));
        }
    }
    (num_indices, num_vertices)
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
                    binding: 0,
                    location: 1,
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
