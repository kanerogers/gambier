use std::{collections::HashMap, io::Cursor};

use id_arena::{Arena, Id};
use nalgebra_glm::TMat4;

use crate::{texture::Texture, vertex::Vertex, vulkan_context::VulkanContext};

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
pub struct Material {
    pub base_colour: Texture,
}

pub struct ImportState<'a> {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    index_offset: u32,
    vertex_offset: u32,
    buffers: Vec<&'a [u8]>,
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
        buffers: Vec<&'a [u8]>,
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

pub fn import_models(vulkan_context: &VulkanContext) -> (Vec<Model>, Arena<Mesh>, Arena<Material>) {
    let gltf = gltf::Gltf::open("assets/NewSponza_Main_Blender_glTF.glb").unwrap();
    let buffer = gltf.blob.as_ref().unwrap().as_slice();
    let buffers = vec![buffer];
    let images = Vec::new();
    let mut import_state = ImportState::new(buffers, images, vulkan_context);

    for material in gltf.materials() {
        if let Some(index) = material.index() {
            if let Some(material) = import_material(material, &mut import_state) {
                let id = import_state.materials.alloc(material);
                import_state.material_ids.insert(index, id);
            }
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

    (
        import_state.models,
        import_state.meshes,
        import_state.materials,
    )
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
        if let Some(material) = import_state.material_ids.get(&material_index).cloned() {
            primitives.push(Primitive {
                index_offset: import_state.index_offset,
                vertex_offset: import_state.vertex_offset,
                num_indices,
                material,
            });
        }

        import_state.index_offset += num_indices;
        import_state.vertex_offset += num_vertices;
    }
}

fn import_material(material: gltf::Material, import_state: &mut ImportState) -> Option<Material> {
    if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
        match texture.texture().source().source() {
            gltf::image::Source::View { view, mime_type } => {
                let buffer = import_state.buffers[0];
                let offset = view.offset();
                let length = view.length();
                let data = &buffer[offset..offset + length];

                let mut image = image::io::Reader::new(Cursor::new(data));
                image.set_format(image::ImageFormat::Png);
                let image = image.decode().unwrap();
                let base_colour = unsafe { Texture::new(import_state.vulkan_context, image) };
                Some(Material { base_colour })
            }
            _ => None,
        }
    } else {
        None
    }
}

fn import_geometry(primitive: &gltf::Primitive, import_state: &mut ImportState) -> (u32, u32) {
    let buffers = &import_state.buffers;
    let reader = primitive.reader(|b| Some(&buffers[b.index()]));
    let mut num_indices = 0;
    for i in reader.read_indices().unwrap().into_u32() {
        num_indices += 1;
        import_state.indices.push(i);
    }

    let mut positions = Vec::new();
    for position in reader.read_positions().unwrap() {
        positions.push(position);
    }
    let num_vertices = positions.len() as _;

    let mut normals = Vec::new();
    if let Some(normal_reader) = reader.read_normals() {
        for normal in normal_reader {
            let normal = [normal[0], normal[1] * -1., normal[2]];
            normals.push(normal);
        }
    } else {
        for _ in 0..num_vertices {
            normals.push([0., 0., 0.]);
        }
    }

    let mut uvs = Vec::new();
    if let Some(tex_coords) = reader.read_tex_coords(0) {
        for uv in tex_coords.into_f32() {
            uvs.push(uv);
        }
    } else {
        for _ in 0..num_vertices {
            uvs.push([0., 0.]);
        }
    }

    let mut colours = Vec::new();
    if let Some(colour_reader) = reader.read_colors(0) {
        for colour in colour_reader.into_rgb_f32() {
            colours.push(colour);
        }
    } else {
        for _ in 0..num_vertices {
            colours.push([0., 0., 0.]);
        }
    }

    for (((position, uv), colour), normal) in positions.drain(..).zip(uvs).zip(colours).zip(normals)
    {
        import_state.vertices.push(Vertex {
            position,
            colour,
            normal,
            uv,
        })
    }

    (num_indices, num_vertices)
}
