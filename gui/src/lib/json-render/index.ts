/**
 * json-render Integration
 *
 * Exports all json-render related functionality for UI previews.
 */

export { catalog } from './catalog';
export { registry } from './registry';
export {
  generatePreview,
  regeneratePreview,
  generatePreviewFromPRD,
  exportTreeAsJSON,
  importTreeFromJSON,
} from './preview';
export type { UIPreview } from '../api';
