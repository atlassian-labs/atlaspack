import '@atlaskit/css-reset';
import {Root as PageLayoutRoot} from '@atlassian/navigation-system/layout/root';
import {TopBar} from '@atlassian/navigation-system/layout/top-bar';
import {
  SideNav,
  SideNavContent,
  SideNavToggleButton,
} from '@atlassian/navigation-system/layout/side-nav';
import AppProvider from '@atlaskit/app-provider';
import {HomeActions, NavLogo} from '@atlassian/navigation-system/top-nav';
import {Main, MenuLinkItem, PanelSplitter} from '@atlassian/navigation-system';
// @ts-ignore
import atlaspackBadge from './badge-light.png';
import {Outlet, useNavigate} from 'react-router';
import HomeIcon from '@atlaskit/icon/glyph/home';
import CacheData from '@atlaskit/icon/glyph/component';
import PageIcon from '@atlaskit/icon/glyph/page';
import {useState} from 'react';

function Logo() {
  return (
    <div style={{display: 'flex', alignItems: 'center', gap: 8}}>
      <img src={atlaspackBadge} alt="Atlaspack" />
      <span
        style={{
          fontSize: 20,
          fontWeight: 600,
          color: 'black',
        }}
      >
        Atlaspack
      </span>
    </div>
  );
}

function LinkItem({
  href,
  children,
  elemBefore,
}: {
  href: string;
  children: React.ReactNode;
  elemBefore: React.ReactNode;
}) {
  const navigate = useNavigate();
  return (
    // @ts-ignore
    <MenuLinkItem
      href={href}
      onClick={(e) => {
        e.preventDefault();
        navigate(href);
      }}
      elemBefore={elemBefore}
    >
      {children}
    </MenuLinkItem>
  );
}

export function AppLayout() {
  const navigate = useNavigate();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  return (
    <AppProvider>
      <PageLayoutRoot>
        <TopBar>
          <HomeActions>
            <SideNavToggleButton
              defaultCollapsed={false}
              expandLabel="Expand sidebar"
              collapseLabel="Collapse sidebar"
              onClick={() => {
                setSidebarCollapsed(!sidebarCollapsed);
              }}
            />

            <NavLogo
              href="/"
              onClick={(e) => {
                e.preventDefault();
                navigate('/');
              }}
              logo={Logo}
              icon={Logo}
              label={''}
            />
          </HomeActions>
        </TopBar>

        {sidebarCollapsed ? null : (
          <SideNav>
            {/* @ts-ignore */}
            <SideNavContent>
              <LinkItem
                elemBefore={
                  // @ts-ignore
                  <HomeIcon label="Home" />
                }
                href="/"
              >
                Cache statistics
              </LinkItem>

              <LinkItem
                elemBefore={
                  // @ts-ignore
                  <CacheData label="Cache data" />
                }
                href="/app/cache"
              >
                Cache data
              </LinkItem>

              <LinkItem
                elemBefore={
                  // @ts-ignore
                  <PageIcon label="Page" />
                }
                href="/app/foamtreemap"
              >
                Bundle size data
              </LinkItem>
            </SideNavContent>

            <PanelSplitter label="Resize side nav" />
          </SideNav>
        )}

        <Main>
          <Outlet />
        </Main>
      </PageLayoutRoot>
    </AppProvider>
  );
}
