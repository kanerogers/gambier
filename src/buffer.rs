use core::ptr::copy_nonoverlapping;
use std::{ffi::c_void, marker::PhantomData};

use ash::{vk, Device, Instance};

pub struct Buffer<T: Sized> {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub memory_address: std::ptr::NonNull<c_void>,
    pub descriptor_set: vk::DescriptorSet,
    _phantom: PhantomData<T>,
}

impl<T: Sized> Buffer<T> {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        data: &[T],
    ) -> Buffer<T> {
        let size = (std::mem::size_of::<T>() * 1024 * 1024) as vk::DeviceSize;
        println!("Attempting to create buffer of {:?} bytes..", size);
        let buffer = device
            .create_buffer(
                &vk::BufferCreateInfo::builder()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                    .size(size),
                None,
            )
            .unwrap();
        println!("..done! Allocating memory..");

        // Allocate memory
        let memory_requirements = device.get_buffer_memory_requirements(buffer);
        let memory_type_bits = memory_requirements.memory_type_bits;
        let memory_properties = instance.get_physical_device_memory_properties(*physical_device);

        let mut memory_type_index = !0;
        for i in 0..memory_properties.memory_type_count as usize {
            if (memory_type_bits & (1 << i)) == 0 {
                continue;
            }
            let properties = memory_properties.memory_types[i].property_flags;
            if properties.contains(
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ) {
                memory_type_index = i;
                println!("Using {} which has flags {:?}", i, properties);
                break;
            }
        }

        if memory_type_index == !0 {
            panic!("Unable to find suitable memory!")
        }

        let device_memory = device
            .allocate_memory(
                &vk::MemoryAllocateInfo::builder()
                    .allocation_size(size)
                    .memory_type_index(memory_type_index as _),
                None,
            )
            .unwrap();

        println!("..done! Binding..");

        // Bind memory
        device.bind_buffer_memory(buffer, device_memory, 0).unwrap();

        println!("..done!");

        // Map memory
        let memory_address = device
            .map_memory(device_memory, 0, size, vk::MemoryMapFlags::empty())
            .unwrap();

        println!("Copying vertices..");
        copy_nonoverlapping(
            data.as_ptr(),
            std::mem::transmute(memory_address),
            data.len(),
        );
        println!("..done!");

        // let descriptor_set = device
        //     .allocate_descriptor_sets(
        //         &vk::DescriptorSetAllocateInfo::builder()
        //             .descriptor_pool(*descriptor_pool)
        //             .set_layouts(&[*descriptor_set_layout]),
        //     )
        //     .unwrap()[0];

        // let buffer_info = DescriptorBufferInfo::builder()
        //     .buffer(buffer)
        //     .offset(0)
        //     .range(std::mem::size_of_val(&vertices) as _);
        // let write = vk::WriteDescriptorSet::builder()
        //     .buffer_info(std::slice::from_ref(&buffer_info))
        //     .dst_set(descriptor_set)
        //     .dst_binding(0)
        //     .descriptor_type(vk::DescriptorType::STORAGE_BUFFER);

        // device.update_descriptor_sets(std::slice::from_ref(&write), &[]);

        Buffer {
            buffer,
            device_memory,
            memory_address: std::ptr::NonNull::new_unchecked(memory_address),
            descriptor_set: vk::DescriptorSet::null(),
            _phantom: PhantomData,
        }
    }
}
