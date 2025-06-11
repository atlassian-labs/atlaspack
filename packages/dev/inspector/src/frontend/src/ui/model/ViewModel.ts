import {makeAutoObservable} from 'mobx';

export interface ViewModel {
  data: BundleData | null;
  focusedBundle: Group | null;
  focusedGroup: Group | null;
  relatedBundles: RelatedBundles | null;
  hasDetails: boolean;
  showLeftSidebar: boolean;
  showRightSidebar: boolean;
  tooltipState: TooltipState | null;
  mouseState: {x: number; y: number};
  groupsById: Map<string, Group>;
}

export const viewModel: ViewModel = makeAutoObservable({
  data: null,
  focusedBundle: null,
  focusedGroup: null,
  relatedBundles: null,
  hasDetails: false,
  showLeftSidebar: true,
  showRightSidebar: false,
  tooltipState: null,
  mouseState: {x: 0, y: 0},

  get groupsById(): Map<string, Group> {
    const groupsById = new Map<string, Group>();

    function collectGroups(group: Group) {
      groupsById.set(group.id, group);
      for (const childGroup of group.groups ?? []) {
        collectGroups(childGroup);
      }
    }

    for (const group of this.data?.groups ?? []) {
      collectGroups(group);
    }

    return groupsById;
  },
} as ViewModel);

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
