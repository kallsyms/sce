# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: sce.proto
"""Generated protocol buffer code."""
from google.protobuf.internal import builder as _builder
from google.protobuf import descriptor as _descriptor
from google.protobuf import descriptor_pool as _descriptor_pool
from google.protobuf import symbol_database as _symbol_database
# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()




DESCRIPTOR = _descriptor_pool.Default().AddSerializedFile(b'\n\tsce.proto\x12\x03sce\"\"\n\x05Point\x12\x0c\n\x04line\x18\x01 \x01(\r\x12\x0b\n\x03\x63ol\x18\x02 \x01(\r\";\n\x05Range\x12\x19\n\x05start\x18\x01 \x01(\x0b\x32\n.sce.Point\x12\x17\n\x03\x65nd\x18\x02 \x01(\x0b\x32\n.sce.Point\"X\n\x06Source\x12\x10\n\x08\x66ilename\x18\x01 \x01(\t\x12\x0f\n\x07\x63ontent\x18\x02 \x01(\t\x12\x10\n\x08language\x18\x03 \x01(\t\x12\x19\n\x05point\x18\x04 \x01(\x0b\x32\n.sce.Point\"S\n\x0cSliceRequest\x12\x1b\n\x06source\x18\x01 \x01(\x0b\x32\x0b.sce.Source\x12&\n\tdirection\x18\x02 \x01(\x0e\x32\x13.sce.SliceDirection\".\n\rSliceResponse\x12\x1d\n\tto_remove\x18\x01 \x03(\x0b\x32\n.sce.Range\"f\n\rInlineRequest\x12\x1b\n\x06source\x18\x01 \x01(\x0b\x32\x0b.sce.Source\x12\x16\n\x0etarget_content\x18\x02 \x01(\t\x12 \n\x0ctarget_point\x18\x03 \x01(\x0b\x32\n.sce.Point\"!\n\x0eInlineResponse\x12\x0f\n\x07\x63ontent\x18\x01 \x01(\t*+\n\x0eSliceDirection\x12\x0c\n\x08\x42\x41\x43KWARD\x10\x00\x12\x0b\n\x07\x46ORWARD\x10\x01\x32h\n\x03SCE\x12.\n\x05Slice\x12\x11.sce.SliceRequest\x1a\x12.sce.SliceResponse\x12\x31\n\x06Inline\x12\x12.sce.InlineRequest\x1a\x13.sce.InlineResponseb\x06proto3')

_builder.BuildMessageAndEnumDescriptors(DESCRIPTOR, globals())
_builder.BuildTopDescriptorsAndMessages(DESCRIPTOR, 'sce_pb2', globals())
if _descriptor._USE_C_DESCRIPTORS == False:

  DESCRIPTOR._options = None
  _SLICEDIRECTION._serialized_start=477
  _SLICEDIRECTION._serialized_end=520
  _POINT._serialized_start=18
  _POINT._serialized_end=52
  _RANGE._serialized_start=54
  _RANGE._serialized_end=113
  _SOURCE._serialized_start=115
  _SOURCE._serialized_end=203
  _SLICEREQUEST._serialized_start=205
  _SLICEREQUEST._serialized_end=288
  _SLICERESPONSE._serialized_start=290
  _SLICERESPONSE._serialized_end=336
  _INLINEREQUEST._serialized_start=338
  _INLINEREQUEST._serialized_end=440
  _INLINERESPONSE._serialized_start=442
  _INLINERESPONSE._serialized_end=475
  _SCE._serialized_start=522
  _SCE._serialized_end=626
# @@protoc_insertion_point(module_scope)
