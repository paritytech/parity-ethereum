// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import React, { PropTypes } from 'react';
import { Step } from 'semantic-ui-react';

import styles from '../title.css';

export default function Steps ({ activeStep, steps }) {
  if (!steps || steps.length < 2) {
    return null;
  }

  return (
    <div className={ styles.steps }>
      <Step.Group ordered>
        {
          steps.map((label, index) => (
            <Step
              completed={ activeStep > index }
              active={ activeStep === index }
              key={ label.key || index }
            >
              <Step.Content>
                <Step.Title>{ label }</Step.Title>
              </Step.Content>
            </Step>
          ))
        }
      </Step.Group>
    </div>
  );
}

Steps.propTypes = {
  activeStep: PropTypes.number,
  steps: PropTypes.array
};
