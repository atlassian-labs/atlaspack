/** @jsx jsx */
import { jsx, cssMap, styled } from '@compiled/react';

const titleStyles = cssMap({ 
  root: { marginBottom: '8px' } 
});

const FeatureCard = styled.div({
  marginTop: '24px',
});

const ButtonContainer = styled.div({
  marginTop: '16px',
});

function FeatureCardView({ title, description }) {
  return (
    <FeatureCard>
      <div css={titleStyles.root}>
        <h3>{title}</h3>
      </div>
      <div>{description}</div>
      <ButtonContainer>
        <button>Learn More</button>
      </ButtonContainer>
    </FeatureCard>
  );
}

export default FeatureCardView;