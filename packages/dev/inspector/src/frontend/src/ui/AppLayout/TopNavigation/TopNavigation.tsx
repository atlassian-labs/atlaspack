import '@atlaskit/css-reset';
import {TopBar} from '@atlassian/navigation-system/layout/top-bar';
import {SideNavToggleButton} from '@atlassian/navigation-system/layout/side-nav';
import {HomeActions, NavLogo} from '@atlassian/navigation-system/top-nav';
import {useNavigate} from 'react-router';
import {useCallback} from 'react';
import {Logo} from './Logo';

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
    <TopBar>
      <HomeActions>
        <SideNavToggleButton
          defaultCollapsed={false}
          expandLabel="Expand sidebar"
          collapseLabel="Collapse sidebar"
          onClick={onClickSideNavToggleButton}
        />

        <NavLogo
          href="/"
          onClick={onClickLogo}
          logo={Logo}
          icon={Logo}
          label={''}
        />
      </HomeActions>
    </TopBar>
  );
}
