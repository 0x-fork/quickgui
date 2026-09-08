// Wire values match the retained Rust host; no JSON tree is rebuilt per frame.

///|
pub(all) enum Value {
  Clear
  Boolean(Bool)
  Number(Float)
  Color(UInt)
  Text(String)
} derive(Eq)

///|
pub struct Batch {
  buffer : @buffer.Buffer
  mut count : Int
}

///|
pub fn batch() -> Batch {
  { buffer: @buffer.Buffer(), count: 0, }
}

///|
pub fn Batch::is_empty(self : Batch) -> Bool {
  self.count == 0
}

///|
pub fn Batch::mutation_count(self : Batch) -> Int {
  self.count
}

///|
fn Batch::op(self : Batch, opcode : Byte) -> Unit {
  if self.count >= MAX_MUTATIONS || self.buffer.length() + 10 >= MAX_BATCH_BYTES {
    abort("QuickGUI mutation batch exceeded its limit")
  }
  self.count += 1
  self.buffer.write_byte(opcode)
}

///|
fn write_text(buffer : @buffer.Buffer, text : String) -> Unit {
  let bytes = @utf8.encode(text)
  if bytes.length() > MAX_STRING_BYTES {
    abort("QuickGUI string exceeded the core limit")
  }
  buffer.write_int_le(bytes.length())
  buffer.write_bytes(bytes)
}

///|
pub fn Batch::create_element(self : Batch, id : Int, tag : Int) -> Unit {
  self.op(b'\x01')
  self.buffer.write_int_le(id)
  self.buffer.write_byte(tag.to_byte())
}

///|
pub fn Batch::create_text(self : Batch, id : Int, value : String) -> Unit {
  self.op(b'\x02')
  self.buffer.write_int_le(id)
  write_text(self.buffer, value)
}

///|
pub fn Batch::create_sentinel(self : Batch, id : Int) -> Unit {
  self.op(b'\x03')
  self.buffer.write_int_le(id)
}

///|
pub fn Batch::set_property(
  self : Batch,
  id : Int,
  property : Int,
  value : Value,
) -> Unit {
  self.op(b'\x04')
  self.buffer.write_int_le(id)
  self.buffer.write_uint16_le(property.to_uint16())
  match value {
    Clear => self.buffer.write_byte(b'\x00')
    Boolean(value) => {
      self.buffer.write_byte(b'\x01')
      self.buffer.write_byte(if value { b'\x01' } else { b'\x00' })
    }
    Number(value) => {
      if value.is_nan() || value.is_inf() {
        abort("QuickGUI numeric properties must be finite")
      }
      self.buffer.write_byte(b'\x02')
      self.buffer.write_float_le(value)
    }
    Color(value) => {
      self.buffer.write_byte(b'\x03')
      self.buffer.write_uint_le(value)
    }
    Text(value) => {
      self.buffer.write_byte(b'\x04')
      write_text(self.buffer, value)
    }
  }
}

///|
pub fn Batch::replace_text(self : Batch, id : Int, value : String) -> Unit {
  self.op(b'\x05')
  self.buffer.write_int_le(id)
  write_text(self.buffer, value)
}

///|
pub fn Batch::insert(
  self : Batch,
  parent : Int,
  child : Int,
  before? : Int = -1,
) -> Unit {
  self.op(b'\x06')
  self.buffer.write_int_le(parent)
  self.buffer.write_int_le(child)
  self.buffer.write_int_le(before)
}

///|
pub fn Batch::remove(self : Batch, parent : Int, child : Int) -> Unit {
  self.op(b'\x07')
  self.buffer.write_int_le(parent)
  self.buffer.write_int_le(child)
}

///|
pub fn Batch::cleanup(
  self : Batch,
  parent : Int,
  children : Array[Int],
) -> Unit {
  self.op(b'\x08')
  self.buffer.write_int_le(parent)
  self.buffer.write_int_le(children.length())
  for child in children {
    self.buffer.write_int_le(child)
  }
}

///|
pub fn Batch::finish(self : Batch) -> Bytes {
  if self.is_empty() {
    return b""
  }
  if self.buffer.length() + 10 > MAX_BATCH_BYTES {
    abort("QuickGUI mutation batch exceeded the core limit")
  }
  let out = @buffer.Buffer(size_hint=self.buffer.length() + 10)
  out.write_bytes(b"QGMB")
  out.write_uint16_le(VERSION.to_uint16())
  out.write_int_le(self.count)
  out.write_bytes(self.buffer.to_bytes())
  self.buffer.reset()
  self.count = 0
  out.to_bytes()
}
