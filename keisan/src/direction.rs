use crate::{Angle, Vector2, Vector2i};

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Invalid = -1,
    South = 0,
    SouthEast = 1,
    East = 2,
    NorthEast = 3,
    North = 4,
    NorthWest = 5,
    West = 6,
    SouthWest = 7,
}

impl Direction {
    pub fn from_index(index: i32) -> Self {
        match index {
            0 => Self::South,
            1 => Self::SouthEast,
            2 => Self::East,
            3 => Self::NorthEast,
            4 => Self::North,
            5 => Self::NorthWest,
            6 => Self::West,
            7 => Self::SouthWest,
            _ => Self::Invalid,
        }
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionFlag {
    None = 0,
    South = 1 << 0,
    East = 1 << 1,
    North = 1 << 2,
    West = 1 << 3,
    SouthEast = (1 << 0) | (1 << 1),
    NorthEast = (1 << 2) | (1 << 1),
    NorthWest = (1 << 2) | (1 << 3),
    SouthWest = (1 << 0) | (1 << 3),
}

pub trait DirectionOps {
    fn to_direction(self) -> Direction;
    fn to_flag(self) -> DirectionFlag;
    fn get_opposite(self) -> Direction;
    fn get_clockwise_90_degrees(self) -> Direction;
    fn to_angle(self) -> Angle;
    fn to_vec(self) -> Vector2;
    fn to_int_vec(self) -> Vector2i;
}

impl DirectionOps for DirectionFlag {
    fn to_direction(self) -> Direction {
        match self {
            DirectionFlag::South => Direction::South,
            DirectionFlag::SouthEast => Direction::SouthEast,
            DirectionFlag::East => Direction::East,
            DirectionFlag::NorthEast => Direction::NorthEast,
            DirectionFlag::North => Direction::North,
            DirectionFlag::NorthWest => Direction::NorthWest,
            DirectionFlag::West => Direction::West,
            DirectionFlag::SouthWest => Direction::SouthWest,
            _ => panic!("invalid direction flag"),
        }
    }

    fn to_flag(self) -> DirectionFlag {
        self
    }

    fn get_opposite(self) -> Direction {
        self.to_direction().get_opposite()
    }

    fn get_clockwise_90_degrees(self) -> Direction {
        self.to_direction().get_clockwise_90_degrees()
    }

    fn to_angle(self) -> Angle {
        self.to_direction().to_angle()
    }

    fn to_vec(self) -> Vector2 {
        self.to_direction().to_vec()
    }

    fn to_int_vec(self) -> Vector2i {
        self.to_direction().to_int_vec()
    }
}

impl DirectionOps for Direction {
    fn to_direction(self) -> Direction {
        self
    }

    fn to_flag(self) -> DirectionFlag {
        match self {
            Direction::South => DirectionFlag::South,
            Direction::SouthEast => DirectionFlag::SouthEast,
            Direction::East => DirectionFlag::East,
            Direction::NorthEast => DirectionFlag::NorthEast,
            Direction::North => DirectionFlag::North,
            Direction::NorthWest => DirectionFlag::NorthWest,
            Direction::West => DirectionFlag::West,
            Direction::SouthWest => DirectionFlag::SouthWest,
            Direction::Invalid => panic!("invalid direction"),
        }
    }

    fn get_opposite(self) -> Direction {
        match self {
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::NorthEast => Direction::SouthWest,
            Direction::SouthWest => Direction::NorthEast,
            Direction::NorthWest => Direction::SouthEast,
            Direction::SouthEast => Direction::NorthWest,
            Direction::Invalid => panic!("invalid direction"),
        }
    }

    fn get_clockwise_90_degrees(self) -> Direction {
        match self {
            Direction::East => Direction::South,
            Direction::West => Direction::North,
            Direction::North => Direction::East,
            Direction::South => Direction::West,
            Direction::NorthEast => Direction::SouthEast,
            Direction::SouthWest => Direction::NorthWest,
            Direction::NorthWest => Direction::NorthEast,
            Direction::SouthEast => Direction::SouthWest,
            Direction::Invalid => panic!("invalid direction"),
        }
    }

    fn to_angle(self) -> Angle {
        let mut ang = 2.0 * core::f64::consts::PI / 8.0 * self as i8 as f64;
        if ang > core::f64::consts::PI {
            ang -= 2.0 * core::f64::consts::PI;
        }
        Angle::from(ang)
    }

    fn to_vec(self) -> Vector2 {
        match self {
            Direction::South => Vector2::new(0.0, -1.0),
            Direction::SouthEast => Vector2::new(1.0, -1.0).normalized(),
            Direction::East => Vector2::new(1.0, 0.0),
            Direction::NorthEast => Vector2::new(1.0, 1.0).normalized(),
            Direction::North => Vector2::new(0.0, 1.0),
            Direction::NorthWest => Vector2::new(-1.0, 1.0).normalized(),
            Direction::West => Vector2::new(-1.0, 0.0),
            Direction::SouthWest => Vector2::new(-1.0, -1.0).normalized(),
            Direction::Invalid => panic!("invalid direction"),
        }
    }

    fn to_int_vec(self) -> Vector2i {
        match self {
            Direction::South => Vector2i::new(0, -1),
            Direction::SouthEast => Vector2i::new(1, -1),
            Direction::East => Vector2i::new(1, 0),
            Direction::NorthEast => Vector2i::new(1, 1),
            Direction::North => Vector2i::new(0, 1),
            Direction::NorthWest => Vector2i::new(-1, 1),
            Direction::West => Vector2i::new(-1, 0),
            Direction::SouthWest => Vector2i::new(-1, -1),
            Direction::Invalid => panic!("invalid direction"),
        }
    }
}

pub trait VectorDirectionOps {
    fn get_dir(self) -> Direction;
    fn get_cardinal_dir(self) -> Direction;
    fn to_angle(self) -> Angle;
    fn to_world_angle(self) -> Angle;
    fn offset(self, dir: Direction) -> Vector2i;
}

impl VectorDirectionOps for Vector2 {
    fn get_dir(self) -> Direction {
        Angle::from_world_vec(self).get_dir()
    }

    fn get_cardinal_dir(self) -> Direction {
        Angle::from_world_vec(self).get_cardinal_dir()
    }

    fn to_angle(self) -> Angle {
        Angle::from(self)
    }

    fn to_world_angle(self) -> Angle {
        Angle::from_world_vec(self)
    }

    fn offset(self, dir: Direction) -> Vector2i {
        Vector2i::from(self) + dir.to_int_vec()
    }
}

impl VectorDirectionOps for Vector2i {
    fn get_dir(self) -> Direction {
        Angle::from(self).get_dir()
    }

    fn get_cardinal_dir(self) -> Direction {
        Angle::from(self).get_cardinal_dir()
    }

    fn to_angle(self) -> Angle {
        Angle::from(self)
    }

    fn to_world_angle(self) -> Angle {
        Angle::from_world_vec(self.into())
    }

    fn offset(self, dir: Direction) -> Vector2i {
        self + dir.to_int_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, DirectionFlag, DirectionOps, VectorDirectionOps};
    use crate::Vector2i;

    #[test]
    fn direction_roundtrips_to_flags() {
        assert_eq!(
            Direction::NorthWest.to_flag().to_direction(),
            Direction::NorthWest
        );
    }

    #[test]
    fn int_offset_matches_cardinal_step() {
        assert_eq!(
            Vector2i::new(4, 5).offset(Direction::West),
            Vector2i::new(3, 5)
        );
        assert_eq!(DirectionFlag::SouthEast.to_int_vec(), Vector2i::new(1, -1));
    }
}
