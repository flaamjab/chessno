use erupt::{DeviceLoader, InstanceLoader};

pub trait DeviceResource {
    fn destroy(&self, device: &DeviceLoader);
}

pub trait InstanceResource {
    fn destroy(&self, instance: &InstanceLoader);
}
