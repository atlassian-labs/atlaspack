import '@testing-library/jest-dom';
import {render, screen} from '@testing-library/react';
import {DefaultLoadingIndicator} from './DefaultLoadingIndicator';

describe('DefaultLoadingIndicator', () => {
  it('should render', () => {
    render(<DefaultLoadingIndicator />);
    expect(screen.getByText('Loading cache stats...')).toBeInTheDocument();
  });

  it('renders provided message', () => {
    render(<DefaultLoadingIndicator message="Loading..." />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
