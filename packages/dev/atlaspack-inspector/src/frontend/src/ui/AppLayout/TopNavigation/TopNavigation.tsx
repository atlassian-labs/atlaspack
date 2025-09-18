import '@atlaskit/css-reset';
import {SideNavToggleButton} from '@atlaskit/navigation-system/layout/side-nav';
import {TopNav, TopNavStart} from '@atlaskit/navigation-system/layout/top-nav';
import {Link, useNavigate} from 'react-router';
import {useCallback} from 'react';
import {Logo} from './Logo';
import * as styles from './TopNavigation.module.css';

interface TopNavigationProps {
  setSidebarCollapsed: (update: (collapsed: boolean) => boolean) => void;
}

export function TopNavigation({setSidebarCollapsed}: TopNavigationProps) {
  const navigate = useNavigate();

  const onClickLogo = useCallback(
    (e: React.MouseEvent<HTMLAnchorElement>) => {
      e.preventDefault();
      navigate('/');
    },
    [navigate],
  );
  const onClickSideNavToggleButton = useCallback(() => {
    setSidebarCollapsed((collapsed) => !collapsed);
  }, [setSidebarCollapsed]);

  return (
    <TopNav>
      <TopNavStart>
        <SideNavToggleButton
          defaultCollapsed={false}
          expandLabel="Expand sidebar"
          collapseLabel="Collapse sidebar"
          onClick={onClickSideNavToggleButton}
        />

        <div className={styles.logoContainer}>
          <Link to="/" onClick={onClickLogo}>
            <Logo />
          </Link>
        </div>
      </TopNavStart>
    </TopNav>
  );
}
