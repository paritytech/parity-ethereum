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

import EventListener from 'react-event-listener';
import React, { Component, PropTypes } from 'react';
import ReactDOM from 'react-dom';
import ReactPortal from 'react-portal';
import keycode from 'keycode';

import { nodeOrStringProptype } from '~/util/proptypes';
import { Title as ContainerTitle } from '~/ui/Container';
import { CloseIcon } from '~/ui/Icons';
import ParityBackground from '~/ui/ParityBackground';

import styles from './portal.css';

export default class Portal extends Component {
  static propTypes = {
    onClose: PropTypes.func.isRequired,
    open: PropTypes.bool.isRequired,
    buttons: PropTypes.array,
    children: PropTypes.node,
    className: PropTypes.string,
    isChildModal: PropTypes.bool,
    onKeyDown: PropTypes.func,
    title: nodeOrStringProptype()
  };

  render () {
    const { children, className, isChildModal, open } = this.props;

    if (!open) {
      return null;
    }

    return (
      <ReactPortal
        isOpened
        onClose={ this.handleClose }
      >
        <div
          className={ styles.backOverlay }
          onClick={ this.handleClose }
        >
          <div
            className={
              [
                styles.overlay,
                isChildModal
                  ? styles.popover
                  : styles.modal,
                className
              ].join(' ')
            }
            onClick={ this.stopEvent }
            onKeyDown={ this.handleKeyDown }
          >
            <EventListener
              target='window'
              onKeyUp={ this.handleKeyUp }
            />
            <ParityBackground className={ styles.parityBackground } />
            <CloseIcon
              className={ styles.closeIcon }
              onClick={ this.handleClose }
            />
            { this.renderTitle() }
            { children }
            { this.renderButtons() }
          </div>
        </div>
      </ReactPortal>
    );
  }

  renderButtons () {
    const { buttons } = this.props;

    if (!buttons) {
      return null;
    }

    return (
      <div className={ styles.buttonRow }>
        { buttons }
      </div>
    );
  }

  renderTitle () {
    const { title } = this.props;

    if (!title) {
      return null;
    }

    return (
      <ContainerTitle
        className={ styles.titleRow }
        title={ title }
      />
    );
  }

  stopEvent = (event) => {
    event.preventDefault();
    event.stopPropagation();
  }

  handleClose = () => {
    this.props.onClose();
  }

  handleKeyDown = (event) => {
    const { onKeyDown } = this.props;

    event.persist();

    return onKeyDown
      ? onKeyDown(event)
      : false;
  }

  handleKeyUp = (event) => {
    const codeName = keycode(event);

    switch (codeName) {
      case 'esc':
        event.preventDefault();
        return this.handleClose();
    }
  }

  handleDOMAction = (ref, method) => {
    const element = ReactDOM.findDOMNode(
      typeof ref === 'string'
        ? this.refs[ref]
        : ref
    );

    if (!element || typeof element[method] !== 'function') {
      console.warn('could not find', ref, 'or method', method);
      return;
    }

    return element[method]();
  }
}
