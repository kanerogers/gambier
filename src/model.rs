use std::{collections::HashMap, io::Cursor};

use ash::vk;
use id_arena::{Arena, Id};
use nalgebra_glm::{vec4, Mat4, Quat, TMat4, Vec3, Vec4};

use crate::{
    buffer::Buffer,
    texture::{create_scratch_buffer, Texture},
    vertex::Vertex,
    vulkan_context::VulkanContext,
};

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub parent_transform: Mat4,
    pub mesh: Id<Mesh>,
}

impl Model {
    pub fn new(
        name: String,
        translation: Vec3,
        rotation: Quat,
        scale: Vec3,
        parent_transform: nalgebra_glm::TMat4<f32>,
        mesh: Id<Mesh>,
    ) -> Self {
        Self {
            name,
            translation,
            rotation,
            scale,
            parent_transform,
            mesh,
        }
    }

    pub(crate) fn get_model_data(&self, mesh: &Mesh) -> ModelData {
        let translation = nalgebra_glm::translate(&self.parent_transform, &self.translation);
        let rotation = nalgebra_glm::quat_to_mat4(&self.rotation);
        let scale = nalgebra_glm::scale(&nalgebra_glm::identity(), &self.scale);
        let transform = translation * rotation * scale;
        let max_scale = scale.max();

        ModelData {
            transform,
            sphere_centre: mesh.sphere_centre + &self.translation,
            sphere_radius: mesh.sphere_radius * max_scale,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub primitives: Vec<Primitive>,
    pub name: String,
    pub sphere_centre: Vec3,
    pub sphere_radius: f32,
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub index_offset: u32,
    pub vertex_offset: u32,
    pub num_indices: u32,
    pub material_id: u16,
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct ModelData {
    pub transform: TMat4<f32>,
    pub sphere_centre: Vec3,
    pub sphere_radius: f32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone)]
pub struct Material {
    pub base_color_factor: Vec4,
    pub base_color_texture_id: u16,
    pub unlit: u16,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color_factor: vec4(1., 1., 1., 1.),
            base_color_texture_id: u16::MAX,
            unlit: 0,
        }
    }
}

pub struct ImportState<'a> {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    index_offset: u32,
    vertex_offset: u32,
    buffers: Vec<&'a [u8]>,
    models: Vec<Model>,
    vulkan_context: &'a VulkanContext,
    meshes: Arena<Mesh>,
    mesh_ids: HashMap<usize, Id<Mesh>>,
    materials: Vec<Material>,
    scratch_buffer: Buffer<u8>,
    textures: Vec<Texture>,
}

impl<'a> ImportState<'a> {
    pub fn new(buffers: Vec<&'a [u8]>, vulkan_context: &'a VulkanContext) -> Self {
        let scratch_buffer = unsafe { create_scratch_buffer(vulkan_context, 1024 * 1024 * 100) };
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            models: Vec::new(),
            index_offset: 0,
            vertex_offset: 0,
            buffers,
            vulkan_context,
            meshes: Arena::new(),
            mesh_ids: HashMap::new(),
            materials: Vec::new(),
            scratch_buffer,
            textures: Vec::new(),
        }
    }
}

pub struct ModelContext {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    pub meshes: Arena<Mesh>,
}

pub fn import_models(vulkan_context: &VulkanContext) -> ModelContext {
    // let gltf = gltf::Gltf::open("assets/sponza.glb").unwrap();
    let gltf = gltf::Gltf::open("assets/test.glb").unwrap();
    let buffer = gltf.blob.as_ref().unwrap().as_slice();
    let buffers = vec![buffer];
    let mut import_state = ImportState::new(buffers, vulkan_context);

    for material in gltf.materials() {
        if let Some(_index) = material.index() {
            import_material(material, &mut import_state);
        }
    }

    for image in gltf.images() {
        import_image(image, &mut import_state);
    }

    for mesh in gltf.meshes() {
        import_mesh(mesh, &mut import_state);
    }

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            import_node(node, &mut import_state, &nalgebra_glm::identity());
        }
    }

    unsafe {
        upload_models(&import_state);
    };

    ModelContext {
        models: import_state.models,
        meshes: import_state.meshes,
        materials: import_state.materials,
    }
}

fn import_mesh(mesh: gltf::Mesh, import_state: &mut ImportState) {
    let mut primitives = Vec::new();
    println!("Importing mesh {}", mesh.index());
    let mut points = Vec::new();
    for primitive in mesh.primitives() {
        import_primitive(primitive, &mut primitives, import_state, &mut points);
    }

    let (sphere_centre, sphere_radius) = get_bounding_sphere(&points);

    let name = mesh
        .name()
        .unwrap_or(&format!("Mesh {}", mesh.index()))
        .to_string();
    let id = import_state.meshes.alloc(Mesh {
        primitives,
        name,
        sphere_centre,
        sphere_radius,
    });
    import_state.mesh_ids.insert(mesh.index(), id);
}

fn import_image(image: gltf::Image, import_state: &mut ImportState) {
    if !image.name().unwrap().contains("BaseColor") {
        import_state.textures.push(Texture {
            image_descriptor_info: vk::DescriptorImageInfo {
                sampler: import_state.vulkan_context.sampler,

                ..Default::default()
            },
        });
        println!("Not importing texture {:?}", image.name());
        return;
    }

    match image.source() {
        gltf::image::Source::View { view, .. } => {
            let buffer = import_state.buffers[0];
            let offset = view.offset();
            let length = view.length();
            let data = &buffer[offset..offset + length];

            let mut image = image::io::Reader::new(Cursor::new(data));
            image.set_format(image::ImageFormat::Png);
            let image = image.decode().unwrap();
            let base_colour_texture = unsafe {
                Texture::new(
                    import_state.vulkan_context,
                    &import_state.scratch_buffer,
                    image,
                )
            };
            import_state.textures.push(base_colour_texture);
        }
        _ => {}
    }
}

fn import_material(material: gltf::Material, import_state: &mut ImportState) {
    let mut new_material = Material::default();

    if let Some(texture) = material.pbr_metallic_roughness().base_color_texture() {
        new_material.base_color_texture_id = texture.texture().source().index() as u16;
    }

    new_material.base_color_factor = material.pbr_metallic_roughness().base_color_factor().into();

    import_state.materials.push(new_material);
}

fn import_node(
    node: gltf::Node,
    import_state: &mut ImportState,
    parent_transform: &nalgebra_glm::TMat4<f32>,
) {
    let local_transform: TMat4<f32> = node.transform().matrix().into();
    let (translation, rotation, scale) = node.transform().decomposed();
    let transform = parent_transform * &local_transform;
    let name = if let Some(name) = node.name() {
        name.to_string()
    } else {
        format!("Node {}", node.index())
    };

    let mesh_id = if let Some(mesh) = node.mesh() {
        import_state.mesh_ids.get(&mesh.index()).unwrap().clone()
    } else {
        import_state.meshes.alloc(Mesh {
            primitives: Vec::new(),
            name: "Empty".to_string(),
            sphere_centre: Vec3::zeros(),
            sphere_radius: 0.,
        })
    };

    if node.mesh().is_some() {
        import_state.models.push(Model::new(
            name,
            translation.into(),
            rotation.into(),
            scale.into(),
            parent_transform.clone(),
            mesh_id,
        ));
    }

    for node in node.children() {
        import_node(node, import_state, &transform);
    }
}

fn import_primitive(
    primitive: gltf::Primitive,
    primitives: &mut Vec<Primitive>,
    import_state: &mut ImportState,
    points: &mut Vec<Vec3>,
) {
    println!("Importing primitive {}", primitive.index());
    let (num_indices, num_vertices) = import_geometry(&primitive, import_state, points);
    let material_id = primitive.material().index().unwrap() as _;
    println!(
        "Primitive has material {} importing geometry..",
        material_id
    );
    primitives.push(Primitive {
        index_offset: import_state.index_offset,
        vertex_offset: import_state.vertex_offset,
        num_indices,
        material_id,
    });
    import_state.index_offset += num_indices;
    import_state.vertex_offset += num_vertices;
    println!("Done - imported {} indices", num_indices);
}

fn import_geometry(
    primitive: &gltf::Primitive,
    import_state: &mut ImportState,
    points: &mut Vec<Vec3>,
) -> (u32, u32) {
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
        points.push(position.into());
    }
    let num_vertices = positions.len() as _;

    let mut normals = Vec::new();
    if let Some(normal_reader) = reader.read_normals() {
        for normal in normal_reader {
            let normal = [normal[0], normal[1], normal[2]];
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

    for ((position, uv), normal) in positions.drain(..).zip(uvs).zip(normals) {
        import_state.vertices.push(Vertex {
            position,
            normal,
            uv,
        })
    }

    (num_indices, num_vertices)
}

fn get_bounding_sphere(points: &[Vec3]) -> (Vec3, f32) {
    let mut centre = Vec3::zeros();
    if points.len() == 0 {
        return (centre, 0.);
    }

    for p in points {
        centre += p;
    }

    centre /= points.len() as f32;
    let mut radius = nalgebra_glm::distance2(&points[0], &centre);
    for p in points.iter().skip(1) {
        radius = radius.max(nalgebra_glm::distance2(p, &centre));
    }

    radius = next_up(radius.sqrt());

    (centre, radius)
}

unsafe fn upload_models(import_state: &ImportState) {
    let vulkan_context = import_state.vulkan_context;

    // Copy indices and vertices into buffers.
    vulkan_context.index_buffer.overwrite(&import_state.indices);
    vulkan_context
        .vertex_buffer
        .overwrite(&import_state.vertices);

    // Copy material data into material buffer.
    vulkan_context
        .material_buffer
        .overwrite(&import_state.materials);

    let image_info = import_state
        .textures
        .iter()
        .map(|t| t.image_descriptor_info)
        .collect::<Vec<_>>();

    if image_info.len() > 0 {
        // Write texture descriptor sets
        let texture_write = vk::WriteDescriptorSet::builder()
            .image_info(&image_info)
            .dst_binding(4)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_array_element(0)
            .dst_set(vulkan_context.shared_descriptor_set);

        vulkan_context
            .device
            .update_descriptor_sets(std::slice::from_ref(&texture_write), &[]);
    }

    // Clean up the scratch buffer
    let device = &vulkan_context.device;
    let scratch_buffer = &import_state.scratch_buffer;
    scratch_buffer.destroy(device);
}

const TINY_BITS: u32 = 0x1; // Smallest positive f32.
const CLEAR_SIGN_MASK: u32 = 0x7fff_ffff;

pub fn next_up(n: f32) -> f32 {
    let bits = n.to_bits();
    if n.is_nan() || bits == f32::INFINITY.to_bits() {
        return n;
    }

    let abs = bits & CLEAR_SIGN_MASK;
    let next_bits = if abs == 0 {
        TINY_BITS
    } else if bits == abs {
        bits + 1
    } else {
        bits - 1
    };
    f32::from_bits(next_bits)
}
