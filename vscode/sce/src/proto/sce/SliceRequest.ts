// Original file: ../../sce.proto

import type { Source as _sce_Source, Source__Output as _sce_Source__Output } from '../sce/Source';
import type { SliceDirection as _sce_SliceDirection } from '../sce/SliceDirection';

export interface SliceRequest {
  'source'?: (_sce_Source | null);
  'direction'?: (_sce_SliceDirection | keyof typeof _sce_SliceDirection);
}

export interface SliceRequest__Output {
  'source': (_sce_Source__Output | null);
  'direction': (_sce_SliceDirection);
}
