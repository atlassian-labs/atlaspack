import {LinkMenuItem} from '@atlaskit/navigation-system';
import {useCallback} from 'react';
import {useNavigate} from 'react-router';

interface LinkItemProps {
  href: string;
  children: React.ReactNode;
  elemBefore: React.ReactNode;
}

export function LinkItem({href, children, elemBefore}: LinkItemProps) {
  const navigate = useNavigate();
  const onClick = useCallback(
    (e: React.MouseEvent<HTMLAnchorElement>) => {
      e.preventDefault();
      navigate(href);
    },
    [href, navigate],
  );

  return (
    <LinkMenuItem href={href} onClick={onClick} elemBefore={elemBefore}>
      {children}
    </LinkMenuItem>
  );
}
