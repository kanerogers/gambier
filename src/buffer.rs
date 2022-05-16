use core::ptr::copy_nonoverlapping;

use ash::{vk, Device, Instance};

use crate::memory::allocate_memory;

pub struct Buffer<T: Sized> {
    pub buffer: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub memory_address: std::ptr::NonNull<T>,
    pub len: usize,
    pub usage: vk::BufferUsageFlags,
}

impl<T: Sized> Buffer<T> {
    pub unsafe fn new(
        device: &Device,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        initial_data: &[T],
        usage: vk::BufferUsageFlags,
        len: usize,
    ) -> Buffer<T> {
        let size = (std::mem::size_of::<T>() * len) as _;
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

        // Transmute the pointer into GPU memory so that we can easily access it again.
        let memory_address = std::mem::transmute(memory_address);

        // Get the

        Buffer {
            buffer,
            device_memory,
            memory_address: std::ptr::NonNull::new_unchecked(memory_address),
            len: initial_data.len(),
            usage,
        }
    }

    /// Dumb update - overrides the content of the GPU buffer with `data`.
    pub unsafe fn overwrite(&self, data: &[T]) {
        copy_nonoverlapping(data.as_ptr(), self.memory_address.as_ptr(), data.len());
    }

    /// safety: After calling this function the buffer will be in an UNUSABLE state
    pub unsafe fn destroy(&self, device: &ash::Device) {
        device.unmap_memory(self.device_memory);
        device.free_memory(self.device_memory, None);
        device.destroy_buffer(self.buffer, None);
    }

    pub unsafe fn update_descriptor_set(
        &mut self,
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        binding: usize,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(self.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let write = vk::WriteDescriptorSet::builder()
            .buffer_info(std::slice::from_ref(&buffer_info))
            .dst_set(descriptor_set)
            .dst_binding(binding as _)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER);

        device.update_descriptor_sets(std::slice::from_ref(&write), &[]);
    }
}
