import {observer} from 'mobx-react-lite';
import {useEffect} from 'react';
import {useSearchParams} from 'react-router';
import {viewModel} from '../../../model/ViewModel';
import {runInAction} from 'mobx';

export const UrlFocusController = observer(() => {
  const [searchParams] = useSearchParams();
  const focusedBundleId = searchParams.get('focusedBundleId');
  const focusedGroupId = searchParams.get('focusedGroupId');

  useEffect(() => {
    if (focusedBundleId !== viewModel.focusedBundle?.id) {
      runInAction(() => {
        viewModel.focusedBundle = focusedBundleId
          ? (viewModel.groupsById.get(focusedBundleId) ?? null)
          : null;
      });
    }
  }, [focusedBundleId]);

  useEffect(() => {
    if (focusedGroupId !== viewModel.focusedGroup?.id) {
      runInAction(() => {
        viewModel.focusedGroup = focusedGroupId
          ? (viewModel.groupsById.get(focusedGroupId) ?? null)
          : null;
      });
    }
  }, [focusedGroupId]);

  return null;
});
