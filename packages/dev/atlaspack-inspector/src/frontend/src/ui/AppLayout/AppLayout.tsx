import '@atlaskit/css-reset';
import {Root as PageLayoutRoot} from '@atlaskit/navigation-system/layout/root';
import AppProvider from '@atlaskit/app-provider';
import {Main} from '@atlaskit/navigation-system';
import {Outlet} from 'react-router';
import {useState} from 'react';
import {TopNavigation} from './TopNavigation/TopNavigation';
import {SidebarNavigation} from './SidebarNavigation/SidebarNavigation';

export function AppLayout() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  return (
    <AppProvider>
      <PageLayoutRoot>
        <TopNavigation setSidebarCollapsed={setSidebarCollapsed} />

        {sidebarCollapsed ? null : <SidebarNavigation />}

        <Main>
          <Outlet />
        </Main>
      </PageLayoutRoot>
    </AppProvider>
  );
}
