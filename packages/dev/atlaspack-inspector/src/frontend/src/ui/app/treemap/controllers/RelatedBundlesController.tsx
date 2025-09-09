import {useSearchParams} from 'react-router';
import {useQuery} from '@tanstack/react-query';
import {observer} from 'mobx-react-lite';
import {useEffect} from 'react';
import {runInAction} from 'mobx';
import {RelatedBundles, viewModel} from '../../../model/ViewModel';
import qs from 'qs';

export const RelatedBundlesController = observer(() => {
  const [searchParams] = useSearchParams();
  const focusedBundleId = searchParams.get('focusedBundleId');
  const {data} = useQuery<RelatedBundles>({
    queryKey: [
      '/api/bundle-graph/related-bundles?' +
        qs.stringify({
          bundle: focusedBundleId,
        }),
    ],
    enabled: focusedBundleId != null,
  });

  useEffect(() => {
    if (data && data.childBundles.length > 0) {
      runInAction(() => {
        viewModel.relatedBundles = data;
      });
    }
  }, [data]);

  const searchParamsBundle = searchParams.get('bundle');
  useEffect(() => {
    if (searchParamsBundle != null) {
      runInAction(() => {
        viewModel.relatedBundles = null;
        viewModel.hasDetails = true;
      });
    }
  }, [searchParamsBundle]);

  return null;
});
