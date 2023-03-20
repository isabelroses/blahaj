const { SlashCommandBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('clear')
        .setDescription('Deletes a number from the channel.')
        .addStringOption(option => option.setName('NumberToDelete').setDescription('Number of messages to delete').setRequired(true)),
    async execute(client, message, input) {
        if (!message.member.hasPermission('MANAGE_MESSAGES')) {
            return message.channel
                .send(
                    "You cant use this command since you're missing `manage_messages` perm",
                );
        }

        if (isNaN(input)) {
            return message.channel
                .send('enter the amount of messages that you would like to clear')
                .then((sent) => {
                    setTimeout(() => {
                        sent.delete();
                    }, 2500);
                });
        }

        if (Number(input) < 0) {
            return message.channel
                .send('enter a positive number')
                .then((sent) => {
                    setTimeout(() => {
                        sent.delete();
                    }, 2500);
                });
        }

        // add an extra to delete the current message too
        const amount = Number(input) > 100
            ? 101
            : Number(input) + 1;

        message.channel.bulkDelete(amount, true)
            .then((_message) => {
                message.channel
                    // do you want to include the current message here?
                    // if not it should be ${_message.size - 1}
                    .send(`Bot cleared \`${_message.size}\` messages :broom:`)
                    .then((sent) => {
                        setTimeout(() => {
                            sent.delete();
                        }, 2500);
                    });
            });
    }
}