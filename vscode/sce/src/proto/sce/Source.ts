// Original file: ../../sce.proto

import type { Point as _sce_Point, Point__Output as _sce_Point__Output } from '../sce/Point';

export interface Source {
  'filename'?: (string);
  'content'?: (string);
  'language'?: (string);
  'point'?: (_sce_Point | null);
}

export interface Source__Output {
  'filename': (string);
  'content': (string);
  'language': (string);
  'point': (_sce_Point__Output | null);
}
