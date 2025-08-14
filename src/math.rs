
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Vec2<T: Copy>(pub T, pub T);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Vec3<T: Copy>(pub T, pub T, pub T);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Vec4<T: Copy>(pub T, pub T, pub T, pub T);


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Mat2<T: Copy>(pub Vec2<T>, pub Vec2<T>);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Mat3<T: Copy>(pub Vec3<T>, pub Vec3<T>, pub Vec3<T>);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Mat4<T: Copy>(pub Vec4<T>, pub Vec4<T>, pub Vec4<T>, pub Vec4<T>);
