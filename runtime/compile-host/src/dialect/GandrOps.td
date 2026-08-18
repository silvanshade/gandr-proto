//===- GandrOps.td - the gandr positive-core dialect -----------*- tablegen -*-===//
//
// The dialect declares the six positive-core transitions the compile host
// lowers, over one opaque value type. Two properties are declared here rather
// than left to a pass: a constructor's arity is checked against its tag, and
// duplication and discard carry memory effects so no canonicalization may
// delete accounted work.
//
//===----------------------------------------------------------------------===//

#ifndef GANDR_OPS_TD
#define GANDR_OPS_TD

include "mlir/IR/AttrTypeBase.td"
include "mlir/IR/OpBase.td"
include "mlir/Interfaces/SideEffectInterfaces.td"

def Gandr_Dialect : Dialect {
  let name = "gandr";
  let cppNamespace = "::gandr::compile_host::dialect";
  let summary = "The gandr positive-core command dialect.";
  let description = [{
    One opaque producer type and the six transitions of the L machine's
    positive core. Consumers that bind — the mu-tilde frame and the constructor
    dispatch — carry their continuations as regions, which is the shape the
    sequent IL has and the shape a flat SSA image does not.
  }];
  let useDefaultTypePrinterParser = 1;
}

class Gandr_Op<string mnemonic, list<Trait> traits = []>
    : Op<Gandr_Dialect, mnemonic, traits>;

class Gandr_Type<string name, string typeMnemonic>
    : TypeDef<Gandr_Dialect, name> {
  let mnemonic = typeMnemonic;
}

def Gandr_ValueType : Gandr_Type<"Value", "val"> {
  let summary = "An opaque positive-core runtime value.";
  let description = [{
    A producer's result. The type is opaque at the dialect level and lowers to
    a machine word addressing a heap cell; keeping it nominal here is what
    stops an unrelated integer from being wired into a consumer, which is the
    part of the Rust representation a flat machine-word image loses.
  }];
}

def Gandr_LitOp : Gandr_Op<"lit", [Pure]> {
  let summary = "An integer literal producer.";
  let description = [{
    The positive scalar leaf. It is pure: a literal carries no accounted work,
    so folding two identical literals into one changes nothing observable.
  }];
  let arguments = (ins SI64Attr:$value);
  let results = (outs Gandr_ValueType:$result);
  let assemblyFormat = "$value attr-dict `:` type($result)";
}

def Gandr_CtorOp : Gandr_Op<"ctor", [Pure]> {
  let summary = "A positive constructor introduction.";
  let description = [{
    The constructor tag declares an arity, and the operation verifier holds the
    operand count to it. The declaration is not the check: an operation built
    with the wrong operand count is representable and is rejected by the
    verifier, never by the builder.
  }];
  let arguments = (ins I32Attr:$tag, Variadic<Gandr_ValueType>:$fields);
  let results = (outs Gandr_ValueType:$result);
  let assemblyFormat = "$tag `(` $fields `)` attr-dict `:` functional-type($fields, $result)";
  let hasVerifier = 1;
}

def Gandr_DupOp : Gandr_Op<"dup", [MemoryEffects<[MemRead, MemWrite]>]> {
  let summary = "The grade structural duplication.";
  let description = [{
    Duplication is an ordinary computation in gandr, not free wiring, so the
    operation is effectful and no canonicalization may delete it. The effect is
    real rather than declarative: the lowering increments the heap's
    duplication ledger, and the regression witness reads that ledger back.
  }];
  let arguments = (ins Gandr_ValueType:$source);
  let results = (outs Gandr_ValueType:$result);
  let assemblyFormat = "$source attr-dict `:` functional-type($source, $result)";
}

def Gandr_DropOp : Gandr_Op<"drop", [MemoryEffects<[MemRead, MemWrite]>]> {
  let summary = "The grade structural discard.";
  let description = [{
    Discard is priced for the same reason duplication is, and carries the same
    effect declaration. Its result is the unit value, so a program that
    discards the discard still executes the accounted work.
  }];
  let arguments = (ins Gandr_ValueType:$source);
  let results = (outs Gandr_ValueType:$result);
  let assemblyFormat = "$source attr-dict `:` functional-type($source, $result)";
}

def Gandr_BindOp : Gandr_Op<"bind", []> {
  let summary = "The sequencing consumer: a mu-tilde binder frame.";
  let description = [{
    The bound producer arrives as an operand and the continuation is a region
    whose single block argument is the binding. Carrying the continuation as a
    region rather than as straight-line SSA is what makes this a consumer
    rather than a let, and it is the cost the lowering pays back.
  }];
  let arguments = (ins Gandr_ValueType:$bound);
  let results = (outs Gandr_ValueType:$result);
  let regions = (region SizedRegion<1>:$body);
  let assemblyFormat =
      "$bound attr-dict-with-keyword $body `:` functional-type($bound, $result)";
  let hasVerifier = 1;
}

def Gandr_CaseOp : Gandr_Op<"case", []> {
  let summary = "A constructor dispatch consumer.";
  let description = [{
    Two arms over the sum injections, each a region whose single block argument
    is the injected payload. The dispatch reads the scrutinee's cell tag; an
    unmatched tag is unreachable by construction on a well-formed image, and
    the lowering routes it to a poison value rather than inventing an answer.
  }];
  let arguments = (ins Gandr_ValueType:$scrutinee);
  let results = (outs Gandr_ValueType:$result);
  let regions = (region SizedRegion<1>:$leftArm, SizedRegion<1>:$rightArm);
  let assemblyFormat = [{
    $scrutinee attr-dict-with-keyword `left` $leftArm `right` $rightArm
    `:` functional-type($scrutinee, $result)
  }];
  let hasVerifier = 1;
}

def Gandr_YieldOp : Gandr_Op<"yield", [Pure, Terminator,
    ParentOneOf<["::gandr::compile_host::dialect::BindOp",
                 "::gandr::compile_host::dialect::CaseOp"]>]> {
  let summary = "The terminator of a consumer region.";
  let arguments = (ins Gandr_ValueType:$result);
  let assemblyFormat = "$result attr-dict `:` type($result)";
}

def Gandr_CutOp : Gandr_Op<"cut", [Terminator]> {
  let summary = "The terminal cut against the top-level consumer.";
  let description = [{
    The one cut this slice realizes: a producer met by the top-level consumer.
    Cuts against a reified continuation are outside the slice, and there is no
    operation here that could express one.
  }];
  let arguments = (ins Gandr_ValueType:$produced);
  let assemblyFormat = "$produced attr-dict `:` type($produced)";
}

#endif // GANDR_OPS_TD
