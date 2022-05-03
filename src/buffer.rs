use core::ptr::copy_nonoverlapping;

use ash::{vk, Device, Instance};

use crate::memory::allocate_memory;

pub struct Buffer<T: Sized> {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub memory_address: std::ptr::NonNull<T>,
    pub descriptor_set: vk::DescriptorSet,
    pub len: usize,
    _usage: vk::BufferUsageFlags,
}

static MAX_LEN: usize = 1024 * 1024;

impl<T: Sized> Buffer<T> {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        initial_data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Buffer<T> {
        let size = std::mem::size_of::<T>() * MAX_LEN;
        let size = size.max(std::mem::size_of::<T>() * initial_data.len()) as vk::DeviceSize;
        println!("Attempting to create buffer of {:?} bytes..", size);
        let buffer = device
            .create_buffer(
                &vk::BufferCreateInfo::builder().usage(usage).size(size),
                None,
            )
            .unwrap();

        println!("..done! Allocating memory..");
        let memory_requirements = device.get_buffer_memory_requirements(buffer);
        let flags = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let device_memory = allocate_memory(
            device,
            instance,
            physical_device,
            memory_requirements,
            flags,
        );

        println!("..done! Binding..");

        // Bind memory
        device.bind_buffer_memory(buffer, device_memory, 0).unwrap();

        println!("..done!");

        // Map memory
        let memory_address = device
            .map_memory(device_memory, 0, size, vk::MemoryMapFlags::empty())
            .unwrap();

        println!("Copying data..");
        copy_nonoverlapping(
            initial_data.as_ptr(),
            std::mem::transmute(memory_address),
            initial_data.len(),
        );
        println!("..done!");

        if usage == vk::BufferUsageFlags::STORAGE_BUFFER {
            let descriptor_set = device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(descriptor_pool)
                        .set_layouts(&[descriptor_set_layout]),
                )
                .unwrap()[0];

            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(buffer)
                .offset(0)
                .range(std::mem::size_of::<T>() as _);
            let write = vk::WriteDescriptorSet::builder()
                .buffer_info(std::slice::from_ref(&buffer_info))
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER);

            device.update_descriptor_sets(std::slice::from_ref(&write), &[]);
        }

        // Transmute the pointer into GPU memory so that we can easily access it again.
        let memory_address = std::mem::transmute(memory_address);

        Buffer {
            buffer,
            device_memory,
            memory_address: std::ptr::NonNull::new_unchecked(memory_address),
            descriptor_set: vk::DescriptorSet::null(),
            len: initial_data.len(),
            _usage: usage,
        }
    }

    /// Dumb update - overrides the content of the GPU buffer with `data`.
    pub unsafe fn overwrite(&self, data: &[T]) {
        copy_nonoverlapping(data.as_ptr(), self.memory_address.as_ptr(), data.len());
    }
}
