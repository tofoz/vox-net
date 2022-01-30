use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::Path,
    sync::{Arc, RwLock},
    thread,
};

//use profiling::register_thread;
use serde::{Deserialize, Serialize};

use noise::{NoiseFn, OpenSimplex};
use prism_math::{min, vec3, xyz_to_index};
use rayon::prelude::*;

use super::layers::Volume;

pub fn noise_3d(x: f32, y: f32, z: f32) -> f32 {
    ((x as f32 * 0.36).sin() + (z as f32 * 0.36).cos() + (y as f32 * 0.36).sin() * 2.0) * 1.0
}

pub fn key_distance(a: (i32, i32, i32), b: (i32, i32, i32)) -> i32 {
    let dis = ((((b.0 - a.0) * (b.0 - a.0))
        + ((b.1 - a.1) * (b.1 - a.1))
        + ((b.2 - a.2) * (b.2 - a.2))) as f32)
        .sqrt();
    dis as i32 // the way we are rounding the number may couse bugs
}

// do not use entitys for per chunk
// one entity is one map with its own chunk managemet

pub struct VoxelMap {
    pub chunk_size: (i32, i32, i32),
    pub chunk_list: BTreeMap<(i32, i32, i32), Chunk>,
}

impl VoxelMap {
    pub fn new(chunk_size: (i32, i32, i32)) -> Self {
        let chunk_list = BTreeMap::new();
        Self {
            chunk_size,
            chunk_list,
        }
    }
    pub fn add_chunk(&mut self, x: i32, y: i32, z: i32, chunk: Chunk) {
        self.chunk_list.insert((x, y, z), chunk);
    }
    pub fn remove_chunk(&mut self, x: i32, y: i32, z: i32) -> Option<Chunk> {
        self.chunk_list.remove(&(x, y, z))
    }
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, val: u16) {
        let key = (x >> 4, y >> 4, z >> 4);
        let local_space = (
            x & (self.chunk_size.0 - 1),
            y & (self.chunk_size.1 - 1),
            z & (self.chunk_size.2 - 1),
        );
        if let Some(c) = self.chunk_list.get_mut(&key) {
            c.set_voxel(
                local_space.0,
                local_space.1,
                local_space.2,
                self.chunk_size.0,
                val,
            );
        }
    }
    pub fn get_voxel(&self, x: i32, y: i32, z: i32) -> u16 {
        // todo later on use optien insted of just returning a u32
        //println!("ki {:?}", (x, y, z));
        let key = (
            (x) >> 4, // rhs 5 means every 32 add 1
            (y) >> 4, // 5 is 32 and 4 is 16? so 3 is 8 and 6 is 64
            (z) >> 4, // mabey use a var that repersents the power of for size of axes like(4,5,3) = x16 y32 z8
        );
        match self.chunk_list.get(&key) {
            Some(c) => {
                let local_space = (
                    x & (self.chunk_size.0 - 1),
                    y & (self.chunk_size.1 - 1),
                    z & (self.chunk_size.2 - 1),
                );

                c.get_voxel(
                    local_space.0,
                    local_space.1,
                    local_space.2,
                    self.chunk_size.0,
                )
            }

            None => 0,
        }
    }
    // mesh faces need to be fliped
    pub fn update_chunk_mesh(
        &self,
        chunk_key: ChunkKey,

        chunk_vertices: &mut Vec<ChunkVertex>,

        mesh_i: &mut Vec<u32>,
        step_i: &mut u32,
        debug_b: bool,
    ) {
        // * is mut so you can set derty flage

        let (ox, oy, oz) = (
            chunk_key.0 .0 * self.chunk_size.0,
            chunk_key.0 .1 * self.chunk_size.1,
            chunk_key.0 .2 * self.chunk_size.2,
        );

        let quad_size = 0.5;
        for x in (ox)..(16 + ox) {
            for y in (oy)..(16 + oy) {
                for z in (oz)..(16 + oz) {
                    let v = self.get_voxel(x, y, z) as u32;

                    // we minus x from ox to translet it back to oriagen
                    let (nx, ny, nz) = if !debug_b {
                        (x - ox, y - oy, z - oz)
                    } else {
                        (x, y, z)
                    };
                    let (nx, ny, nz) = (
                        nx as f32 + quad_size,
                        ny as f32 + quad_size,
                        nz as f32 + quad_size,
                    );

                    if v >= 1 {
                        let gv_up = self.get_voxel(x, y + 1, z);
                        let gv_down = self.get_voxel(x, y - 1, z);
                        let gv_right = self.get_voxel(x + 1, y, z);
                        let gv_left = self.get_voxel(x - 1, y, z);
                        let gv_front = self.get_voxel(x, y, z + 1);
                        let gv_back = self.get_voxel(x, y, z - 1);

                        // basic light
                        let mut ll: i32 = 6;
                        let dr = 26;
                        {
                            let mut dsy: i32 = 0;
                            let mut dey: i32 = 0;
                            //if gv_up == 0 {
                            dey = dr;
                            //}
                            if gv_down == 0 {
                                //dsy = dr;
                            }

                            for dy in (-dsy)..=dey {
                                if dy != 0 {
                                    let tgv = self.get_voxel(x, y + dy, z);
                                    if tgv != 0 {
                                        ll = -dy;
                                    }
                                }
                            }
                        }

                        let fll = ((ll as f32 / 6.0) * 2.0).max(0.5);

                        if gv_up == 0 {
                            add_quad(
                                FaceSide::Up,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }

                        if gv_down == 0 {
                            add_quad(
                                FaceSide::Down,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }

                        if gv_right == 0 {
                            add_quad(
                                FaceSide::Right,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }

                        if gv_left == 0 {
                            add_quad(
                                FaceSide::Left,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }

                        if gv_front == 0 {
                            add_quad(
                                FaceSide::Front,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }

                        if gv_back == 0 {
                            add_quad(
                                FaceSide::Back,
                                [fll, fll, fll],
                                (nx as f32, ny as f32, nz as f32),
                                0.5,
                                chunk_vertices,
                                step_i,
                                mesh_i,
                                v - 1,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub struct ChunkKey((i32, i32, i32));

impl ChunkKey {
    pub fn new(input: (i32, i32, i32)) -> Self {
        ChunkKey(input)
    }
}

/// mint to be a simpleafide chunk for saveing and loading, can run a compreson algarithom on data be for saveing
#[derive(Serialize, Deserialize, Debug)]
pub struct SaveChunk {
    pub voxel: Option<Vec<u32>>,
}

pub struct Chunk {
    pub dirty: bool,
    pub save_dirty: bool, // used to tell when we need to update the mesh
    pub entity_exist: bool,
    pub size: i32,
    pub volume: Option<Volume>, // Option so you dont use up mimory unless there is a voxel in the chunk
}
// todo : refacter to use new volume type
impl Chunk {
    pub fn new(size: i32) -> Self {
        Self {
            volume: None,
            dirty: false,
            save_dirty: false,
            entity_exist: false,
            size,
        }
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn set_is_dirty(&mut self, d: bool) {
        self.dirty = d;
    }
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, size: i32, val: u16) {
        self.save_dirty = true;
        self.dirty = true;
        match &mut self.volume {
            Some(v) => {
                v.type_id.layer[xyz_to_index!(x, y, z, size, size) as usize] = val;
                self.set_is_dirty(true);
            }
            None => {
                // if the insert value is 0/Air we do not make voxel = some

                let vol = Volume::new((self.size * self.size * self.size) as usize);

                self.volume = Some(vol);

                match &mut self.volume {
                    Some(v) => {
                        v.type_id.layer[xyz_to_index!(x, y, z, size, size) as usize] = val;
                        self.set_is_dirty(true);
                    }
                    None => {
                        println!("err set voxel");
                    }
                }
            }
        }
    }
    pub fn get_voxel(&self, x: i32, y: i32, z: i32, size: i32) -> u16 {
        if (z >= size || z < 0) || (y >= size || y < 0) || (x >= size || x < 0) {
            println!("err vox out of bound: {:?}", (x, y, z));
            return 0;
        }

        match &self.volume {
            Some(v) => {
                return v.type_id.layer[xyz_to_index!(x, y, z, size, size) as usize];
            }
            None => 0,
        }
    }
}

pub enum FaceSide {
    Up,
    Down,
    Left,
    Right,
    Front,
    Back,
}

pub fn add_quad(
    side: FaceSide,
    color: [f32; 3],
    pos: (f32, f32, f32),
    s: f32,
    chunk_vertices: &mut Vec<ChunkVertex>,
    i_step: &mut u32,
    vec_i: &mut Vec<u32>,
    tex_index: u32,
) {
    let mut normal = [0.0, 0.0, 0.0];
    match side {
        FaceSide::Up => {
            // done
            normal = [0.0, 1.0, 0.0];
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //        light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //       light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //       light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //         light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(*i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(3 + *i_step);

            vec_i.push(1 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(3 + *i_step);

            *i_step += 4;
        }
        FaceSide::Down => {
            normal = [0.0, -1.0, 0.0];
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //         light: [0.3, 0.3, 0.3, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //          light: [0.3, 0.3, 0.3, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //        light: [0.3, 0.3, 0.3, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //         light: [0.3, 0.3, 0.3, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(3 + *i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(*i_step);

            vec_i.push(3 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(1 + *i_step);
            *i_step += 4;
        }
        FaceSide::Left => {
            normal = [-1.0, 0.0, 0.0];
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //        light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //        light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //        light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(*i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(3 + *i_step);

            vec_i.push(1 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(3 + *i_step);
            *i_step += 4;
        }
        FaceSide::Right => {
            normal = [1.0, 0.0, 0.0];
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //           light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //         light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(*i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(3 + *i_step);

            vec_i.push(1 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(3 + *i_step);
            *i_step += 4;
        }
        FaceSide::Front => {
            // done
            normal = [0.0, 0.0, 1.0];
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //            light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, s + pos.2],
                color,
                normal,
                //           light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(*i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(3 + *i_step);

            vec_i.push(1 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(3 + *i_step);
            *i_step += 4;
        }
        FaceSide::Back => {
            normal = [0.0, 0.0, -1.0];
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //           light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, -s + pos.1, -s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 1.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [-s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //          light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [1.0, 0.0],
                index: tex_index as u16,
            });
            chunk_vertices.push(ChunkVertex {
                position: [s + pos.0, s + pos.1, -s + pos.2],
                color,
                normal,
                //         light: [1.0, 1.0, 1.0, 1.0],
                uv_0: [0.0, 0.0],
                index: tex_index as u16,
            });

            vec_i.push(*i_step);
            vec_i.push(1 + *i_step);
            vec_i.push(3 + *i_step);

            vec_i.push(1 + *i_step);
            vec_i.push(2 + *i_step);
            vec_i.push(3 + *i_step);
            *i_step += 4;
        }
    }
}

// ---- chunk mesh ------------------------------------------
// todo: may whant to pack meta data in a [u8;4] array
// can pack normal data using that technic.
#[derive(Copy, Clone, Default)]
// color and light can be a float and converted to u8 norm , 2x + 2x + 2x = rgb + light_rgb

pub struct ChunkVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// color albedo tent
    pub color: [f32; 3],
    /// rgbi light value
    //pub light: [u8; 4],
    // uvi useing f32 seams wastfull
    // also i index should be u16 or u32
    // u8x2 norm for uv
    //
    /// uv + index for texture array
    pub uv_0: [f32; 2],
    pub index: u16,
}
