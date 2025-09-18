import {makeAutoObservable} from 'mobx';

export class ViewModel {
  data: BundleData | null = null;
  focusedBundle: Group | null = null;
  focusedGroup: Group | null = null;
  relatedBundles: RelatedBundles | null = null;
  hasDetails: boolean = false;
  showLeftSidebar: boolean = true;
  showRightSidebar: boolean = false;
  tooltipState: TooltipState | null = null;
  mouseState: {x: number; y: number} = {x: 0, y: 0};
  bottomPanelHeight: number = 400;

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
  }

  constructor() {
    makeAutoObservable(this);
  }
}

export const viewModel: ViewModel = new ViewModel();

export interface BundleData {
  groups: Array<Group>;
}

export interface RelatedBundles {
  childBundles: Array<{id: string; displayName: string; size: number}>;
}

export type Group = {
  id: string;
  type: 'bundle' | 'asset';
  label: string;
  weight: number;
  groups?: Array<Group>;
  /**
   * If this is a bundle, then the size on disk is diff. than the sum of the assets
   * due to minification.
   */
  assetTreeSize?: number;
};

export interface TooltipState {
  group: Group;
}
