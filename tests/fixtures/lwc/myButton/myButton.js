import { LightningElement, api } from 'lwc';

export default class MyButton extends LightningElement {
    @api label = 'Click Me';
    @api variant;
    @api disabled;

    @api focus() {
        this.template.querySelector('button').focus();
    }

    handleClick() {
        this.dispatchEvent(new CustomEvent('press'));
    }
}
