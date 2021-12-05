use super::{convert, Expr, Instr, Op, Packet, RType, Value, VarBlock};

impl Packet {
  pub fn generate_writer(&mut self) {
    let mut gen = WriterGen::new(&self.reader, self.name.clone());
    gen.instr(&self.reader.block, &mut self.writer.block);
  }
}

struct WriterGen {
  vars:   Vec<Expr>,
  packet: String,
}

impl WriterGen {
  fn new(v: &VarBlock, packet: String) -> Self {
    WriterGen { vars: v.vars.iter().map(|_| Expr::new(Value::Null)).collect(), packet }
  }

  fn instr(&mut self, read: &[Instr], writer: &mut Vec<Instr>) {
    for i in read {
      match i {
        Instr::Set(field, expr) => {
          if let Some(instr) = self.set_expr(expr, field) {
            writer.push(instr);
          }
        }
        Instr::Let(i, expr) => self.vars[*i] = expr.clone(),
        Instr::Return(_) => {}
        Instr::For(_, _range, _) => {}
        Instr::Switch(_, _table) => {}
        Instr::If(cond, when_true, when_false) => {
          let mut when_t = vec![];
          let mut when_f = vec![];
          self.instr(when_true, &mut when_t);
          self.instr(when_false, &mut when_f);
          writer.push(Instr::If(cond.clone(), when_t, when_f));
        }
        _ => panic!("cannot convert {:?} into writer (packet {})", i, self.packet),
      }
    }
  }

  fn set_expr(&mut self, expr: &Expr, field: &str) -> Option<Instr> {
    Some(match expr.ops.first() {
      Some(Op::Call(class, name, _args)) if class == "tcp::Packet" => {
        assert_eq!(expr.initial, Value::Var(1), "unknown Set value: {:?}", expr);
        let writer_name = convert::reader_to_writer(name);
        let mut val = Expr::new(Value::Field(field.into()));
        for op in expr.ops.iter().skip(1) {
          val.ops.push(match op {
            // Convert the cast `foo = buf.read_u8() as i32` into `buf.write_u8(foo as u8)`
            Op::Cast(_from) => {
              // TODO: Find the type of `val`
              let to = RType::new("f32");
              Op::As(to)
            }
            Op::BitAnd(v) => Op::BitAnd(v.clone()),
            Op::Div(v) => Op::Mul(v.clone()),
            _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
          });
        }
        Instr::Expr(Expr::new(Value::Var(1)).op(Op::Call(
          class.clone(),
          writer_name.into(),
          vec![val],
        )))
      }
      Some(Op::If(_cond, _new)) => return None,
      None => return None,
      _ => panic!("cannot convert {:?} into writer (packet {})", expr, self.packet),
    })
  }
}
