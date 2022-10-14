// Original file: ../../sce.proto

import type * as grpc from '@grpc/grpc-js'
import type { MethodDefinition } from '@grpc/proto-loader'
import type { InlineRequest as _sce_InlineRequest, InlineRequest__Output as _sce_InlineRequest__Output } from '../sce/InlineRequest';
import type { InlineResponse as _sce_InlineResponse, InlineResponse__Output as _sce_InlineResponse__Output } from '../sce/InlineResponse';
import type { SliceRequest as _sce_SliceRequest, SliceRequest__Output as _sce_SliceRequest__Output } from '../sce/SliceRequest';
import type { SliceResponse as _sce_SliceResponse, SliceResponse__Output as _sce_SliceResponse__Output } from '../sce/SliceResponse';

export interface SCEClient extends grpc.Client {
  Inline(argument: _sce_InlineRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  Inline(argument: _sce_InlineRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  Inline(argument: _sce_InlineRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  Inline(argument: _sce_InlineRequest, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  inline(argument: _sce_InlineRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  inline(argument: _sce_InlineRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  inline(argument: _sce_InlineRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  inline(argument: _sce_InlineRequest, callback: grpc.requestCallback<_sce_InlineResponse__Output>): grpc.ClientUnaryCall;
  
  Slice(argument: _sce_SliceRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  Slice(argument: _sce_SliceRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  Slice(argument: _sce_SliceRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  Slice(argument: _sce_SliceRequest, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  slice(argument: _sce_SliceRequest, metadata: grpc.Metadata, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  slice(argument: _sce_SliceRequest, metadata: grpc.Metadata, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  slice(argument: _sce_SliceRequest, options: grpc.CallOptions, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  slice(argument: _sce_SliceRequest, callback: grpc.requestCallback<_sce_SliceResponse__Output>): grpc.ClientUnaryCall;
  
}

export interface SCEHandlers extends grpc.UntypedServiceImplementation {
  Inline: grpc.handleUnaryCall<_sce_InlineRequest__Output, _sce_InlineResponse>;
  
  Slice: grpc.handleUnaryCall<_sce_SliceRequest__Output, _sce_SliceResponse>;
  
}

export interface SCEDefinition extends grpc.ServiceDefinition {
  Inline: MethodDefinition<_sce_InlineRequest, _sce_InlineResponse, _sce_InlineRequest__Output, _sce_InlineResponse__Output>
  Slice: MethodDefinition<_sce_SliceRequest, _sce_SliceResponse, _sce_SliceRequest__Output, _sce_SliceResponse__Output>
}
