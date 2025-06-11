import {makeAutoObservable} from 'mobx';

export interface ViewModel {
  focusedBundle: Group | null;
  focusedGroup: Group | null;
  relatedBundles: RelatedBundles | null;
  hasDetails: boolean;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
  tooltipState: TooltipState | null;
  mouseState: {x: number; y: number};
}

export const viewModel: ViewModel = makeAutoObservable({
  focusedBundle: null,
  focusedGroup: null,
  relatedBundles: null,
  hasDetails: false,
  showLeftSidebar: true,
  showRightSidebar: false,
  tooltipState: null,
  mouseState: {x: 0, y: 0},
});

export interface BundleData {
  groups: Array<Group>;
}

export interface RelatedBundles {
  childBundles: Array<{id: string; displayName: string; size: number}>;
}

export interface Group {
  id: string;
  type: 'bundle' | 'asset';
  label: string;
  weight: number;
  groups?: Array<Group>;
}

export interface TooltipState {
  group: Group;
}
