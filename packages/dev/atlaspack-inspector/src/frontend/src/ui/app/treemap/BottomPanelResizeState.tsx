import {makeAutoObservable} from 'mobx';
import {ViewModel} from '../../model/ViewModel';

/**
 * View model for bottom panel UI.
 */
export class BottomPanelResizeState {
  isResizing = false;
  /**
   * Last known Y mouse coordinate
   */
  lastMouseY = 0;
  isHovering = false;
  viewModel: ViewModel;

  constructor(viewModel: ViewModel) {
    this.viewModel = viewModel;
    makeAutoObservable(this);
  }

  startResize = (e: React.MouseEvent) => {
    this.isResizing = true;
    this.lastMouseY = e.clientY;
    document.addEventListener('mousemove', this.onMouseMove);
  };

  mouseEnter = () => {
    this.isHovering = true;
  };

  mouseLeave = () => {
    this.isHovering = false;
  };

  stopResize = () => {
    this.isResizing = false;
    document.removeEventListener('mousemove', this.onMouseMove);
  };

  onMouseMove = (e: MouseEvent) => {
    if (this.isResizing) {
      const deltaY = this.lastMouseY - e.clientY;
      this.viewModel.bottomPanelHeight =
        this.viewModel.bottomPanelHeight + deltaY;
      this.lastMouseY = e.clientY;
    }
  };
}
