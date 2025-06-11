import {observer} from 'mobx-react-lite';
import {Link, useSearchParams} from 'react-router';
import {viewModel} from '../../../../model/ViewModel';
import styles from './FocusBreadcrumbs.module.css';
import qs from 'qs';

export const FocusBreadcrumbs = observer(() => {
  const [searchParams] = useSearchParams();
  const bundleEl = viewModel.focusedBundle ? (
    <Link to={`/app/treemap?bundle=${viewModel.focusedBundle.id}`}>
      {viewModel.focusedBundle.label}
    </Link>
  ) : null;

  const focusedGroup = viewModel.focusedGroup
    ? viewModel.focusedGroup.id.split('/').map((part, i, arr) => {
        const candidatePath = arr.slice(0, i + 1).join('/');
        return (
          <div key={i}>
            <Link
              to={`/app/treemap?${qs.stringify({
                bundle:
                  viewModel.focusedBundle?.id ?? searchParams.get('bundle'),
                focusedBundleId: viewModel.focusedBundle?.id,
                focusedGroupId: candidatePath,
              })}`}
            >
              {part}
            </Link>
          </div>
        );
      })
    : [];

  const breadcrumEls = [
    <Link to="/app/treemap">Root</Link>,
    bundleEl,
    ...focusedGroup,
  ];

  return (
    <div className={styles.focusBreadcrumbs}>
      {breadcrumEls.flatMap((el, i) => [
        <div key={i}>{el}</div>,
        i < breadcrumEls.length - 1 && <div key={i + '-separator'}>&gt;</div>,
      ])}
    </div>
  );
});
