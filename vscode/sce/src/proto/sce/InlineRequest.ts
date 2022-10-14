// Original file: ../../sce.proto

import type { Source as _sce_Source, Source__Output as _sce_Source__Output } from '../sce/Source';
import type { Point as _sce_Point, Point__Output as _sce_Point__Output } from '../sce/Point';

export interface InlineRequest {
  'source'?: (_sce_Source | null);
  'targetContent'?: (string);
  'targetPoint'?: (_sce_Point | null);
}

export interface InlineRequest__Output {
  'source': (_sce_Source__Output | null);
  'targetContent': (string);
  'targetPoint': (_sce_Point__Output | null);
}
