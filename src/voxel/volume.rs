use serde::{Deserialize, Serialize};
use std::{any::Any, collections::HashMap, mem::size_of};

use prism_math::{max, min, println_expression, vec3, Vec3};

// * ---------- 3 points are: volume, attributes, and conversion --------------------
// * volume, holed the data in a raw form as array of u8's.
// * attributes, describe how to interact with the raw data and how its layed out.
// * conversion, refers to converting a volume form one layout to another.

// the mesh'r should ask for the individuals chunks not the hole map
/// used to get a mutable view inside a volume map.

struct VolumeView {}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]

pub struct Volume {
    vol: Vec<Vec<u8>>,
}

impl Volume {
    pub fn get_new_volume(attribute_layout: &AttributeLayout) -> Self {
        let mut v_attributes = Vec::new();
        for (_i, attribute) in attribute_layout.layout.iter().enumerate() {
            v_attributes.push(vec![0_u8; attribute.total_size_in_bytes]);
        }
        Self { vol: v_attributes }
    }
    pub fn get<T>(&self, attribute: usize, offset: usize, length: usize) -> &[T] {
        unsafe {
            let (_lh, output, _rh) = self.vol[attribute]
                [(offset * size_of::<T>())..(length * size_of::<T>())]
                .align_to::<T>();
            output
        }
    }
    pub fn get_mut<T>(&mut self, attribute: usize, offset: usize, length: usize) -> &mut [T] {
        unsafe {
            let (_lh, output, _rh) = self.vol[attribute]
                [(offset * size_of::<T>())..(length * size_of::<T>())]
                .align_to_mut::<T>();
            output
        }
    }
    // todo: fn insert_element's at index() give a range of elements of type T and insert it in the vec
    // todo: fn remove_element's in a given rage()
}

#[derive(Clone, Debug)]
pub enum AttributeFormat {
    U8,
    U8x3,
    U16,
    U32,
}

impl AttributeFormat {
    fn get_size_for(input: Self) -> usize {
        match input {
            AttributeFormat::U8 => size_of::<u8>(),
            AttributeFormat::U8x3 => size_of::<u8>() * 3,
            AttributeFormat::U16 => size_of::<u16>(),
            AttributeFormat::U32 => size_of::<u32>(),
        }
    }
}

// todo: look into using traits for attributes !!
#[derive(Clone, Debug)]
pub struct Attribute {
    /// size if a individual element in bytes.
    bits_per_element: usize,
    /// total volume size in bytes.
    total_size_in_bytes: usize,
    /// data type.
    //attribute_format: AttributeFormat,
    name: String,
}
impl Attribute {
    pub fn new<T>(
        total_size_in_elements: usize,
        //attribute_format: AttributeFormat,
        name: String,
    ) -> Self {
        Self {
            bits_per_element: size_of::<T>(),
            total_size_in_bytes: total_size_in_elements * size_of::<T>(),
            // attribute_format,
            name,
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

// may rename to volume layout
pub struct AttributeLayout {
    pub layout: Vec<Attribute>,
}

#[test]
fn volume_test() {
    {
        let mut vv = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];

        {
            // look at the diferant spliting fuchons

            let (a, mid) = vv.split_at_mut(3);
            let (mid, b) = mid.split_at_mut(2);

            let mut cview = vec![a, b];
            for i in cview.iter_mut() {
                for (i, x) in i.iter_mut().enumerate() {
                    *x += 5;
                }
            }
            println!("cv, {:?}", cview);
        }
        println!("vv, {:?}", vv);
    }

    println_expression!({
        let xx = 5.0 as f64;
        let yy = 193.4234;
        max!(xx, yy)
    });

    // may beabule to store type as a dynamic trait object.
    let layout = AttributeLayout {
        layout: vec![
            Attribute::new::<U8vec3>(4 * 1 * 1, "albedo_compact".to_string()),
            Attribute::new::<U8vec3>(4 * 1 * 1, "normal_compact".to_string()),
        ],
    };

    let layout_v = AttributeLayout {
        layout: vec![Attribute {
            bits_per_element: size_of::<Vec3>(),
            total_size_in_bytes: (4 * 1 * 1) * size_of::<Vec3>(),
            //  attribute_format: AttributeFormat::U16,
            name: "normal".to_string(),
        }],
    };

    let mut vol = Volume::get_new_volume(&layout);
    let mut vol_v = Volume::get_new_volume(&layout_v);

    // fill volume
    for (i, v) in vol
        .get_mut::<U8vec3>(1, 0, 4 * 1 * 1)
        .iter_mut()
        .enumerate()
    {
        v.x += i as u8;
        v.y += v.x + i as u8;
        v.z += v.y + i as u8;
    }

    // convert volume
    let src_normal = vol.get::<U8vec3>(1, 0, 4 * 1 * 1);
    let dest_normal = vol_v.get_mut::<Vec3>(0, 0, 4 * 1 * 1);
    for (i, v) in src_normal.iter().enumerate() {
        let vv = Vec3::from(*v);
        dest_normal[i] = vv;
    }
    println!("{:?}", vol.get::<U8vec3>(1, 0, 4 * 1 * 1));
    println!("{:?}", vol_v.get::<Vec3>(0, 0, 4 * 1 * 1));
}

// --------------------------------------------------------

pub trait TAttribute {
    fn get_align_to<T>(input: &[u8]) -> &[T];
}

impl TAttribute for U8vec3 {
    fn get_align_to<U8vec3>(input: &[u8]) -> &[U8vec3] {
        unsafe {
            let (_l, out, _r) = input.align_to::<U8vec3>();
            return out;
        }
    }
}

// conversion testing

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct U8vec3 {
    x: u8,
    y: u8,
    z: u8,
}

impl U8vec3 {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }
}

impl From<U8vec3> for Vec3 {
    fn from(impute: U8vec3) -> Self {
        let mut v: Vec3 = vec3(impute.x as f32, impute.y as f32, impute.z as f32);
        v *= 1.0 / 255.0;
        v
    }
}

impl From<Vec3> for U8vec3 {
    fn from(mut impute: Vec3) -> Self {
        impute *= 255.0;
        U8vec3::new(impute.x as u8, impute.y as u8, impute.z as u8)
    }
}
