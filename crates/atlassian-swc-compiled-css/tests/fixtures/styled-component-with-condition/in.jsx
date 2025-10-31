import { styled } from '@compiled/react';
import { componentWithCondition } from '@atlassian/jira-feature-flagging-utils';
import { easeInOut } from '@atlaskit/motion';

const OuterWrapperOld = styled.div({
  display: 'flex',
  contain: 'layout',
  width: ({ width }) => width,
  transition: ({ duration }) => `width ${duration}ms ${easeInOut}`,
});

const OuterWrapperNew = styled.div({
  display: 'flex',
  contain: 'layout',
  width: ({ width }) => width,
  transition: ({ duration }) => `width ${duration}ms ${easeInOut}`,
  overflow: 'hidden',
});

const OuterWrapper = componentWithCondition(
  () => true,
  OuterWrapperNew,
  OuterWrapperOld
);

const InnerWrapper = styled.div({
  flexShrink: 0,
});

export const Component = ({ width, duration }) => (
  <OuterWrapper width={width} duration={duration}>
    <InnerWrapper />
  </OuterWrapper>
);
