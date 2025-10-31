import { ClassNames } from '@compiled/react';

export const Component = () => (
  <ClassNames>
    {({ css }) => <div className={css({ fontSize: 12 })} />}
  </ClassNames>
);
