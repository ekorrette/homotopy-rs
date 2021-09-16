// TODO(@doctorn) remove

use homotopy_common::idx::Idx;
use homotopy_core::Generator;
use ultraviolet::Vec4;

use super::geom::{Boundary, Mesh, Vertex, VertexExt};

pub fn example_3() -> Mesh {
    const V_COORDS: [[i32; 4]; 6] = [
        [0, 0, -1, 0],
        [0, -1, 0, 0],
        [-1, 0, 0, 0],
        [0, 0, 1, 0],
        [0, 1, 0, 0],
        [1, 0, 0, 0],
    ];
    const V_BOUNDS: [Boundary; 6] = [Boundary::Two; 6];
    const SQUARES: [[usize; 4]; 8] = [
        [0, 1, 2, 2],
        [1, 3, 2, 2],
        [4, 0, 2, 2],
        [3, 4, 2, 2],
        [1, 0, 5, 5],
        [3, 1, 5, 5],
        [0, 4, 5, 5],
        [4, 3, 5, 5],
    ];

    let mut mesh = Mesh::new();

    for (bound, coord) in V_BOUNDS.iter().zip(V_COORDS.iter()) {
        mesh.mk_vertex(
            Vec4::new(
                coord[0] as f32,
                coord[1] as f32,
                coord[2] as f32,
                coord[3] as f32,
            )
            .with_boundary_and_generator(*bound, Generator::new(0, 0)),
        );
    }

    for square in &SQUARES {
        mesh.mk_element_from(&[
            Vertex::new(square[0]),
            Vertex::new(square[1]),
            Vertex::new(square[2]),
            Vertex::new(square[3]),
        ]);
    }

    mesh
}

pub fn snake_3() -> Mesh {
    const V_COORDS: [[f32; 4]; 10] = [
        [0., 1., -1., 0.],
        [0., 0., -1., 0.],
        [0., -1., -1., 0.],
        [0., 1., 0., 0.],
        [0., 0., 0., 0.],
        [0., -1., 0., 0.],
        [1., 1., 1., 0.],
        [1., -0.75, 1., 0.],
        [-1., 0.75, 1., 0.],
        [-1., -1., 1., 0.],
    ];
    const V_BOUNDS: [Boundary; 10] = [
        Boundary::Zero,
        Boundary::One,
        Boundary::Zero,
        Boundary::One,
        Boundary::Two,
        Boundary::One,
        Boundary::Zero,
        Boundary::One,
        Boundary::One,
        Boundary::Zero,
    ];
    const SQUARES: [[usize; 4]; 5] = [
        [1, 4, 0, 3],
        [2, 5, 1, 4],
        [4, 7, 3, 6],
        [4, 8, 4, 7],
        [9, 8, 5, 4],
    ];

    let mut mesh = Mesh::new();

    for (bound, coord) in V_BOUNDS.iter().zip(V_COORDS.iter()) {
        mesh.mk_vertex(
            Vec4::new(coord[0], coord[1], coord[2], coord[3])
                .with_boundary_and_generator(*bound, Generator::new(0, 0)),
        );
    }

    for square in &SQUARES {
        mesh.mk_element_from(&[
            Vertex::new(square[0]),
            Vertex::new(square[1]),
            Vertex::new(square[2]),
            Vertex::new(square[3]),
        ]);
    }

    mesh
}

pub fn example_4() -> Mesh {
    const V_COORDS: [[i32; 4]; 13] = [
        [0, 0, 0, -1],
        [0, 0, -1, 0],
        [0, -1, 0, 0],
        [-1, 0, 0, 0],
        [0, 0, 0, 1],
        [0, 0, 1, 0],
        [0, 1, 0, 0],
        [1, 0, 0, -1],
        [1, 0, -1, 0],
        [1, -1, 0, 0],
        [1, 0, 0, 1],
        [1, 0, 1, 0],
        [1, 1, 0, 0],
    ];
    const V_BOUNDS: [Boundary; 13] = [
        Boundary::Three,
        Boundary::Three,
        Boundary::Three,
        Boundary::Three,
        Boundary::Three,
        Boundary::Three,
        Boundary::Three,
        Boundary::Two,
        Boundary::Two,
        Boundary::Two,
        Boundary::Two,
        Boundary::Two,
        Boundary::Two,
    ];
    const CUBES: [[usize; 8]; 16] = [
        [3, 3, 3, 3, 0, 1, 2, 2],
        [4, 1, 2, 2, 3, 3, 3, 3],
        [0, 5, 2, 2, 3, 3, 3, 3],
        [3, 3, 3, 3, 4, 5, 2, 2],
        [0, 1, 6, 6, 3, 3, 3, 3],
        [3, 3, 3, 3, 4, 1, 6, 6],
        [3, 3, 3, 3, 0, 5, 6, 6],
        [4, 5, 6, 6, 3, 3, 3, 3],
        [0, 7, 1, 8, 2, 9, 2, 9],
        [2, 9, 2, 9, 4, 10, 1, 8],
        [2, 9, 2, 9, 0, 7, 5, 11],
        [4, 10, 5, 11, 2, 9, 2, 9],
        [6, 12, 6, 12, 0, 7, 1, 8],
        [4, 10, 1, 8, 6, 12, 6, 12],
        [0, 7, 5, 11, 6, 12, 6, 12],
        [6, 12, 6, 12, 4, 10, 5, 11],
    ];

    let mut mesh = Mesh::new();

    for (bound, coord) in V_BOUNDS.iter().zip(V_COORDS.iter()) {
        mesh.mk_vertex(
            Vec4::new(
                coord[0] as f32,
                coord[1] as f32,
                coord[2] as f32,
                coord[3] as f32,
            )
            .with_boundary_and_generator(*bound, Generator::new(0, 0)),
        );
    }

    for cube in &CUBES {
        mesh.mk_element_from(&[
            Vertex::new(cube[0]),
            Vertex::new(cube[1]),
            Vertex::new(cube[2]),
            Vertex::new(cube[3]),
            Vertex::new(cube[4]),
            Vertex::new(cube[5]),
            Vertex::new(cube[6]),
            Vertex::new(cube[7]),
        ]);
    }

    mesh
}
