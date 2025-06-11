import {observer} from 'mobx-react-lite';
import {Link} from 'react-router';
import {runInAction} from 'mobx';
import {viewModel} from '../../../../model/ViewModel';

export const FocusBreadcrumbs = observer(() => {
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
              to={`/app/treemap?bundle=${viewModel.focusedBundle?.id}&path=${candidatePath}`}
              onClick={(e) => {
                // TODO: Make this work
                e.preventDefault();

                runInAction(() => {
                  viewModel.focusedGroup = null;
                });
              }}
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
    <div
      style={{
        padding: '4px',
        display: 'flex',
        flexDirection: 'row',
        gap: '4px',
      }}
    >
      {breadcrumEls.flatMap((el, i) => [
        <div key={i}>{el}</div>,
        i < breadcrumEls.length - 1 && <div>&gt;</div>,
      ])}
    </div>
  );
});
