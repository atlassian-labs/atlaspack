import '../../hack-feature-flags';
import '@testing-library/jest-dom';
import {act, render, screen} from '@testing-library/react';
import {ErrorBoundary} from './ErrorBoundary';
import {makeAutoObservable, runInAction} from 'mobx';
import {observer} from 'mobx-react-lite';

describe('ErrorBoundary', () => {
  beforeEach(() => {
    jest.spyOn(console, 'error').mockImplementation(() => {});
  });
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('should render the main content when no error is thrown', () => {
    render(
      <ErrorBoundary>
        <div>Hello</div>
      </ErrorBoundary>,
    );

    expect(screen.getByText('Hello')).toBeInTheDocument();
  });

  it('should render the fallback component when an error is thrown', () => {
    let state = makeAutoObservable({
      shouldThrow: false,
    });
    let stopThrowing = false;
    const Throws = observer(() => {
      if (state.shouldThrow && !stopThrowing) {
        throw new Error('test error');
      }

      return <div>Hello</div>;
    });

    const Sample = () => {
      return (
        <ErrorBoundary>
          <Throws />
        </ErrorBoundary>
      );
    };

    render(<Sample />);
    act(() => {
      runInAction(() => {
        state.shouldThrow = true;
      });
    });

    expect(screen.getByText('test error')).toBeInTheDocument();

    act(() => {
      stopThrowing = true;
      screen
        .getByTestId('atlaspack-inspector-error-boundary-reset-error')
        .click();
    });

    expect(screen.getByText('Hello')).toBeInTheDocument();
  });
});
