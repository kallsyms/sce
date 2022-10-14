import type * as grpc from '@grpc/grpc-js';
import type { EnumTypeDefinition, MessageTypeDefinition } from '@grpc/proto-loader';

import type { SCEClient as _sce_SCEClient, SCEDefinition as _sce_SCEDefinition } from './sce/SCE';

type SubtypeConstructor<Constructor extends new (...args: any) => any, Subtype> = {
  new(...args: ConstructorParameters<Constructor>): Subtype;
};

export interface ProtoGrpcType {
  sce: {
    InlineRequest: MessageTypeDefinition
    InlineResponse: MessageTypeDefinition
    Point: MessageTypeDefinition
    Range: MessageTypeDefinition
    SCE: SubtypeConstructor<typeof grpc.Client, _sce_SCEClient> & { service: _sce_SCEDefinition }
    SliceDirection: EnumTypeDefinition
    SliceRequest: MessageTypeDefinition
    SliceResponse: MessageTypeDefinition
    Source: MessageTypeDefinition
  }
}

