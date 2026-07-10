// ---------------------- ?? ----------------------------------

use crate::pva_message::{header::MsgEndian, typ::PvaElement};

pub struct PvaVarSizeArr<T: PvaElement> {
    pub arr: Vec<T>,
}

impl<T: PvaElement> PvaVarSizeArr<T> {
    pub fn arr(self: &Self) -> &[T] {
        &self.arr
    }

    pub fn arr_mut(self: &mut Self) -> &mut Vec<T> {
        &mut self.arr
    }

    // pub fn to_buf(self: &Self, endian: MsgEndian) -> Result<Vec<u8>, String> {
    //     let mut buf: Vec<u8> = vec!([]);
    //     self.arr().len().to_buf(&mut buf, endian)?;
    //     for element in self.arr() {
    //         element.to_buf(&mut buf, endian)?;
    //     }
    //     Ok(buf)
    // }

    // pub fn append_to_buf(
    //     self: &Self,
    //     buf: &mut Vec<u8>,
    //     endian: MsgEndian,
    // ) -> Result<usize, String> {
    //     let new_buf = match self.to_buf(endian) {
    //         Ok(new_buf) => new_buf,
    //         Err(err) => return Err(err),
    //     };
    //     buf.extend_from_slice(&new_buf);
    //     Ok(new_buf.len())
    // }

    pub fn from_buf(
        buf: &[u8],
        offset: &mut usize,
        endian: MsgEndian,
    ) -> Result<PvaVarSizeArr<T>, String> {
        let offset_init = *offset;
        if offset_init > buf.len() {
            return Err("Error: PVA array offset past end of buffer".to_string());
        }

        let mut element_offset = offset_init;
        let size = usize::from_buf(buf, &mut element_offset, endian)?;
        let mut arr: Vec<T> = Vec::with_capacity(size);

        for _ in 0..size {
            let element = T::from_buf(buf, &mut element_offset, endian)?;
            arr.push(element);
        }

        *offset = element_offset;

        Ok(PvaVarSizeArr { arr: arr })
    }
}
