import {ViewModel} from '../../model/ViewModel';
import {BottomPanelResizeState} from './BottomPanelResizeState';

describe('BottomPanelResizeState', () => {
  it('should resize the bottom panel', () => {
    const viewModel = new ViewModel();
    viewModel.bottomPanelHeight = 100;
    const resizeState = new BottomPanelResizeState(viewModel);

    resizeState.startResize({clientY: 100} as React.MouseEvent);
    expect(viewModel.bottomPanelHeight).toBe(100);

    resizeState.onMouseMove({clientY: 50} as MouseEvent);
    expect(viewModel.bottomPanelHeight).toBe(150);

    resizeState.onMouseMove({clientY: 30} as MouseEvent);
    expect(viewModel.bottomPanelHeight).toBe(170);

    resizeState.onMouseMove({clientY: 110} as MouseEvent);
    expect(viewModel.bottomPanelHeight).toBe(90);

    resizeState.stopResize();
    expect(viewModel.bottomPanelHeight).toBe(90);
  });
});
