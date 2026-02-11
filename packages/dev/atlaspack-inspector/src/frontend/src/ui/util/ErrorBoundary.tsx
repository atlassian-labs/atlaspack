import {CodeBlock} from '@atlaskit/code';
import {Stack} from '@atlaskit/primitives';
import Button from '@atlaskit/button/new';
import {Component} from 'react';
import * as styles from './ErrorBoundary.module.css';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: (props: {error: Error; resetError: () => void}) => React.ReactNode;
}

function DefaultFallback({
  error,
  resetError,
}: {
  error: Error;
  resetError: () => void;
}) {
  return (
    <Stack
      alignBlock="center"
      alignInline="center"
      space="space.300"
      xcss={{
        height: '100%',
        width: '100%',
      }}
    >
      <h1 className={styles.errorMessage}>
        Error: <span>{error.message}</span>
      </h1>
      <CodeBlock text={error.stack ?? ''} />
      <Button
        appearance="primary"
        onClick={resetError}
        testId="atlaspack-inspector-error-boundary-reset-error"
      >
        Retry loading
      </Button>
    </Stack>
  );
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  {error: Error | null}
> {
  state = {
    error: null as Error | null,
  };

  componentDidCatch(error: Error): void {
    this.setState({error});
  }

  render() {
    if (this.state.error) {
      if (this.props.fallback == null) {
        return (
          <DefaultFallback
            error={this.state.error}
            resetError={() => this.setState({error: null})}
          />
        );
      }

      return this.props.fallback({
        error: this.state.error,
        resetError: () => this.setState({error: null}),
      });
    }

    return this.props.children;
  }
}
