import {useEffect} from 'react';
import {runInAction} from 'mobx';
import {viewModel} from '../../../../model/ViewModel';

/**
 * Sync mouse state into viewModel
 */
export function useMouseMoveController() {
  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      runInAction(() => {
        viewModel.mouseState = {x: e.offsetX, y: e.offsetY};
      });
    };

    window.addEventListener('mousemove', onMouseMove);

    return () => window.removeEventListener('mousemove', onMouseMove);
  }, []);
}
