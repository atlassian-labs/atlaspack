import '@atlaskit/css-reset';
import {
  SideNav,
  SideNavContent,
} from '@atlaskit/navigation-system/layout/side-nav';
import {PanelSplitter} from '@atlaskit/navigation-system';
import HomeIcon from '@atlaskit/icon/glyph/home';
import CacheData from '@atlaskit/icon/glyph/component';
import PageIcon from '@atlaskit/icon/glyph/page';
import {LinkItem} from './LinkItem';

export function SidebarNavigation() {
  return (
    <SideNav>
      <SideNavContent>
        <LinkItem elemBefore={<PageIcon label="Page" />} href="/">
          Bundle size data
        </LinkItem>

        <LinkItem
          elemBefore={<HomeIcon label="Home" />}
          href="/app/cache-stats"
        >
          Cache statistics
        </LinkItem>

        <LinkItem
          elemBefore={<CacheData label="Cache invalidation" />}
          href="/app/cache-invalidation"
        >
          Cache invalidation
        </LinkItem>

        <LinkItem
          elemBefore={<CacheData label="Cache data" />}
          href="/app/cache"
        >
          Cache data
        </LinkItem>
      </SideNavContent>

      <PanelSplitter label="Resize side nav" />
    </SideNav>
  );
}
