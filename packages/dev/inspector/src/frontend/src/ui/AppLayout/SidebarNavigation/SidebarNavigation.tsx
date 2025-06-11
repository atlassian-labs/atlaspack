import '@atlaskit/css-reset';
import {
  SideNav,
  SideNavContent,
} from '@atlassian/navigation-system/layout/side-nav';
import {PanelSplitter} from '@atlassian/navigation-system';
import HomeIcon from '@atlaskit/icon/glyph/home';
import CacheData from '@atlaskit/icon/glyph/component';
import PageIcon from '@atlaskit/icon/glyph/page';
import {LinkItem} from './LinkItem';

export function SidebarNavigation() {
  return (
    <SideNav>
      <SideNavContent>
        <LinkItem elemBefore={<HomeIcon label="Home" />} href="/">
          Cache statistics
        </LinkItem>

        <LinkItem
          elemBefore={<CacheData label="Cache data" />}
          href="/app/cache"
        >
          Cache data
        </LinkItem>

        <LinkItem elemBefore={<PageIcon label="Page" />} href="/app/treemap">
          Bundle size data
        </LinkItem>
      </SideNavContent>

      <PanelSplitter label="Resize side nav" />
    </SideNav>
  );
}
